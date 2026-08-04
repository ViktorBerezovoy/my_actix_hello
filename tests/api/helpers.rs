use actix_hello::configuration::get_configuration;
use actix_hello::startup::Application;
use actix_hello::telemetry::{get_subscriber, init_subscriber};
use sqlx::PgPool;
use std::sync::LazyLock;
use url::Url;
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

static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

pub struct ConfirmationLinks {
    pub html: url::Url,
    pub plain_text: url::Url,
}

pub struct TestApp {
    pub address: String,
    pub email_server: MockServer,
    pub port: u16,
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
}

pub async fn spawn_app(db_pool: &PgPool) -> TestApp {
    LazyLock::force(&TRACING);

    let email_server = MockServer::start().await;
    let mut configuration = get_configuration().expect("Failed to read configuration.");
    configuration.application.port = 0;
    configuration.email_client.base_url = Url::parse(&email_server.uri()).unwrap();
    let application =
        Application::build(configuration, db_pool).expect("Failed to create application");
    let socket_addr = application.address();
    let address = format!("http://{}:{}", socket_addr.ip(), socket_addr.port());
    let port = socket_addr.port();
    tokio::spawn(application.run());

    TestApp {
        address,
        email_server,
        port,
        client: HTTP_CLIENT.clone(),
    }
}
