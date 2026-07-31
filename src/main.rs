use actix_hello::email_client::EmailClient;
use actix_hello::telemetry::{get_subscriber, init_subscriber};
use actix_hello::{configuration::get_configuration, startup::run};
use secrecy::ExposeSecret;
use sqlx::PgPool;
use std::net::TcpListener;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("actix_hello", "info", std::io::stdout);
    init_subscriber(subscriber);

    let configuration = match get_configuration() {
        Ok(value) => value,
        Err(e) => panic!("Failed to load configuration {e}"),
    };
    let connection_pool =
        PgPool::connect_lazy(configuration.database.connection_string().expose_secret())
            .expect("Failed to connect to Postgres.");

    let email_client = EmailClient::new(
        configuration.email_client.base_url,
        configuration.email_client.sender_email,
        configuration.email_client.authorization_token,
        std::time::Duration::from_secs(10),
    );
    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    let listener = TcpListener::bind(address)?;
    run(listener, connection_pool, email_client)?.await
}
