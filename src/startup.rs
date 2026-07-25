use crate::routes::{health_check, subscribe};
use actix_web::{App, HttpServer, dev::Server};
use std::net::TcpListener;

pub fn run(listener: TcpListener) -> Result<Server, std::io::Error> {
    let server = HttpServer::new(|| App::new().service(health_check).service(subscribe))
        .listen(listener)?
        .run();

    Ok(server)
}
