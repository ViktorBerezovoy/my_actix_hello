use std::net::TcpListener;

use actix_hello::{configuration::get_configuration, startup::run};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let configuration = match get_configuration() {
        Ok(value) => value,
        Err(e) => panic!("Failed to load configuration {e}"),
    };
    let address = format!("127.0.0.1:{}", configuration.application_port);
    let listener = TcpListener::bind(address)?;
    run(listener)?.await
}
