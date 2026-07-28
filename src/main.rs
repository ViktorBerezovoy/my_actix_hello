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
        PgPool::connect(configuration.database.connection_string().expose_secret())
            .await
            .expect("Failed to connect to Postgres.");
    let address = format!("127.0.0.1:{}", configuration.application_port);
    let listener = TcpListener::bind(address)?;
    run(listener, connection_pool)?.await
}
