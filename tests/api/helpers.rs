use actix_hello::configuration::get_configuration;
use actix_hello::startup::Application;
use actix_hello::telemetry::{get_subscriber, init_subscriber};
use argon2::password_hash::SaltString;
use argon2::password_hash::rand_core::OsRng;
use argon2::{Argon2, PasswordHasher};
use sqlx::Row;
use sqlx::postgres::PgConnectOptions;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::env;
use std::str::FromStr;
use std::sync::LazyLock;
use tokio::sync::OnceCell;
use url::Url;
use uuid::Uuid;
use wiremock::MockServer;

static TRACING: LazyLock<()> = LazyLock::new(|| {
    let default_filter_level = "info";
    let subscriber_name = "test";
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    }
});

static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .cookie_store(true)
        .build()
        .unwrap()
});

static TEMPLATE_INIT: OnceCell<()> = OnceCell::const_new();

async fn init_template_databese(base_url: &str) {
    let admin_options = PgConnectOptions::from_str(base_url)
        .expect("Failed to parse DATABASE_URL")
        .database("postgres");

    let mut connection = PgConnection::connect_with(&admin_options)
        .await
        .expect("Failed to connect to Postgres");
    let old_dbs = sqlx::query("SELECT datname FROM pg_database WHERE datname LIKE 'test_db_%'")
        .fetch_all(&mut connection)
        .await
        .expect("Failed to fetch old test databases");

    for row in old_dbs {
        let db_name: String = row.get("datname");

        let terminate_query = format!(
            r#"
            SELECT pg_terminate_backend(pg_stat_activity.pid)
            FROM pg_stat_activity
            WHERE pg_stat_activity.datname = '{}' AND pid <> pg_backend_pid();
            "#,
            db_name
        );
        let drop_query = format!(r#"DROP DATABASE "{}";"#, db_name);

        let telemetry_query_static: &'static str = Box::leak(terminate_query.into_boxed_str());
        let drop_query_static: &'static str = Box::leak(drop_query.into_boxed_str());

        connection.execute(telemetry_query_static).await.unwrap();

        connection.execute(drop_query_static).await.unwrap();
    }

    let template_name = "newsletter_test_template";

    let termitane_query = r#"
        SELECT pg_terminate_backend(pg_stat_activity.pid)
        FROM pg_stat_activity
        WHERE pg_stat_activity.datname = 'newsletter_test_template' AND pid <> pg_backend_pid();
        "#;

    let drop_query = r#"DROP DATABASE IF EXISTS "newsletter_test_template";"#;

    let create_query = r#"CREATE DATABASE  "newsletter_test_template";"#;

    connection
        .execute(termitane_query)
        .await
        .expect("Failed to clean up template");
    connection
        .execute(drop_query)
        .await
        .expect("Failed to clean up template");
    connection
        .execute(create_query)
        .await
        .expect("Failed to create database.");

    let template_options = PgConnectOptions::from_str(base_url)
        .expect("Failed to parse DATABASE_URL")
        .database(template_name);

    let template_pool = PgPool::connect_with(template_options)
        .await
        .expect("Failed to connect to template db");

    sqlx::migrate!("./migrations")
        .run(&template_pool)
        .await
        .expect("Failed to migrate temlate database");

    template_pool.close().await;
}

async fn configure_database(test_db_name: &str) -> PgPool {
    dotenvy::dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env file");

    TEMPLATE_INIT
        .get_or_init(|| async {
            init_template_databese(&db_url).await;
        })
        .await;

    let admin_options = PgConnectOptions::from_str(&db_url)
        .expect("Failed to patse DATABASE_URL")
        .database("postgres");

    let mut connection = PgConnection::connect_with(&admin_options)
        .await
        .expect("Failed to connect to admin Postgres");

    let create_db_query = format!(
        r#"CREATE DATABASE "{}" TEMPLATE "newsletter_test_template";"#,
        test_db_name
    );
    let create_db_query_static: &'static str = Box::leak(create_db_query.into_boxed_str());

    connection
        .execute(create_db_query_static)
        .await
        .expect("Failed to clone database from template");

    let test_db_options = PgConnectOptions::from_str(&db_url)
        .expect("Failed to parse DATABASE_URL")
        .database(test_db_name);

    PgPool::connect_with(test_db_options)
        .await
        .expect("Failed to connect to test Postgres")
}

pub struct ConfirmationLinks {
    pub html: url::Url,
    pub plain_text: url::Url,
}

pub struct TestUser {
    pub user_id: Uuid,
    pub username: String,
    pub password: String,
}
impl TestUser {
    pub fn new() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }
    async fn store(&self, pool: &PgPool) {
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(self.password.as_bytes(), &salt)
            .unwrap()
            .to_string();
        sqlx::query!(
            "INSERT INTO users (user_id, username, password_hash) VALUES ($1, $2, $3)",
            self.user_id,
            self.username,
            password_hash,
        )
        .execute(pool)
        .await
        .expect("Failed to create test user.");
    }
}

pub struct TestApp {
    pub address: String,
    pub email_server: MockServer,
    pub port: u16,
    pub db_pool: PgPool,
    client: reqwest::Client,
    pub test_user: TestUser,
    server_handler: tokio::task::JoinHandle<Result<(), anyhow::Error>>,
}
impl TestApp {
    pub async fn post_subscriptions(&self, body: &str) -> reqwest::Response {
        self.client
            .post(format!("{}/subscriptions", self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body.to_string())
            .send()
            .await
            .expect("Failed to execute request.")
    }
    pub fn get_confirmation_link(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();

            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = Url::parse(&raw_link).unwrap();
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_link(body["html"].as_str().unwrap());
        let plain_text = get_link(body["text"].as_str().unwrap());

        ConfirmationLinks { html, plain_text }
    }
    pub async fn post_newsletters(&self, body: &serde_json::Value) -> reqwest::Response {
        self.client
            .post(format!("{}/newsletters", self.address))
            .basic_auth(&self.test_user.username, Some(&self.test_user.password))
            .json(body)
            .send()
            .await
            .expect("Failed to esecute request.")
    }
    pub async fn post_login<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.client
            .post(format!("{}/login", self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }
    pub async fn get_login_html(&self) -> String {
        self.client
            .get(format!("{}/login", self.address))
            .send()
            .await
            .expect("Failed to execute request.")
            .text()
            .await
            .unwrap()
    }
    pub async fn get_admin_dashboard(&self) -> reqwest::Response {
        self.client
            .get(format!("{}/admin/dashboard", self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }
    pub async fn get_admin_dashboard_html(&self) -> String {
        self.get_admin_dashboard().await.text().await.unwrap()
    }
}
impl Drop for TestApp {
    fn drop(&mut self) {
        self.server_handler.abort();
    }
}

pub fn assert_is_redirect_to(response: &reqwest::Response, location: &str) {
    assert_eq!(response.status(), reqwest::StatusCode::SEE_OTHER);
    assert_eq!(response.headers().get("Location").unwrap(), location);
}

pub async fn spawn_app() -> TestApp {
    LazyLock::force(&TRACING);

    let email_server = MockServer::start().await;
    let mut configuration = get_configuration().expect("Failed to read configuration.");
    configuration.application.port = 0;
    configuration.email_client.base_url = Url::parse(&email_server.uri()).unwrap();
    let database_name = format!("test_db_{}", Uuid::new_v4());
    let db_pool = configure_database(&database_name).await;
    let application = Application::build(configuration, &db_pool.clone(), Some(4))
        .await
        .expect("Failed to create application");
    let socket_addr = application.address();
    let address = format!("http://{}:{}", socket_addr.ip(), socket_addr.port());
    let port = socket_addr.port();
    let server_handler = tokio::spawn(application.run());

    let test_user = TestUser::new();
    test_user.store(&db_pool).await;

    TestApp {
        address,
        email_server,
        port,
        client: HTTP_CLIENT.clone(),
        test_user,
        server_handler,
        db_pool,
    }
}
