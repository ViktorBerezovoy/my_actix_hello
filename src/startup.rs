use crate::configuration::Settings;
use crate::email_client::EmailClient;
use crate::routes::{health_check, subscribe};
use actix_web::{App, HttpServer, dev::Server, web};
use sqlx::PgPool;
use std::net::{SocketAddr, TcpListener};
use tracing_actix_web::TracingLogger;

pub struct Application {
    address: SocketAddr,
    server: Server,
}

impl Application {
    pub fn build(configuration: Settings, pg_pool: &PgPool) -> Result<Self, std::io::Error> {
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

        let listener = TcpListener::bind(&address)?;
        let local_address = listener.local_addr()?;
        let server = run(listener, pg_pool.clone(), email_client)?;

        Ok(Self {
            address: local_address,
            server,
        })
    }
    pub fn address(&self) -> &SocketAddr {
        &self.address
    }
    pub async fn run(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
) -> Result<Server, std::io::Error> {
    let db_pool = web::Data::new(db_pool);
    let email_client = web::Data::new(email_client);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .service(health_check)
            .service(subscribe)
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
