use actix_hello::configuration::get_configuration;
use actix_hello::startup::Application;
use actix_hello::telemetry::{get_subscriber, init_subscriber};
use sqlx::PgPool;
use std::sync::LazyLock;

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

static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

pub struct TestApp {
    pub address: String,
    client: reqwest::Client,
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
}

pub async fn spawn_app(db_pool: &PgPool) -> TestApp {
    LazyLock::force(&TRACING);

    let mut configuration = get_configuration().expect("Failed to read configuration.");
    configuration.application.port = 0;
    let application =
        Application::build(configuration, db_pool).expect("Failed to create application");
    let socket_addr = application.address();
    let address = format!("http://{}:{}", socket_addr.ip(), socket_addr.port());
    tokio::spawn(application.run());

    TestApp {
        address,
        client: HTTP_CLIENT.clone(),
    }
}
