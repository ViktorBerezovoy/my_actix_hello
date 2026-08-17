use actix_hello::configuration::get_configuration;
use actix_hello::startup::Application;
use actix_hello::telemetry::{get_subscriber, init_subscriber};
use secrecy::ExposeSecret;
use sqlx::PgPool;

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

    let application = Application::build(configuration, &connection_pool, None)?;
    application.run().await
}
