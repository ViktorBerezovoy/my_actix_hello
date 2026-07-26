use std::net::TcpListener;

use actix_hello::{configuration::get_configuration, startup::run};
use sqlx::PgPool;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let configuration = match get_configuration() {
        Ok(value) => value,
        Err(e) => panic!("Failed to load configuration {e}"),
    };
    let connection_pool = PgPool::connect(&configuration.database.connection_string())
        .await
        .expect("Failed to connect to Postgres.");
    let address = format!("127.0.0.1:{}", configuration.application_port);
    let listener = TcpListener::bind(address)?;
    run(listener, connection_pool)?.await
}
