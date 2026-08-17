use crate::configuration::Settings;
use crate::email_client::EmailClient;
use crate::routes::subscriptions_confirm::confirm;
use crate::routes::{health_check, publish_newsletter, subscribe};
use actix_web::{App, HttpServer, dev::Server, web};
use sqlx::PgPool;
use std::net::{SocketAddr, TcpListener};
use tracing_actix_web::TracingLogger;

pub struct Application {
    address: SocketAddr,
    server: Server,
}

impl Application {
    pub fn build(
        configuration: Settings,
        pg_pool: &PgPool,
        workers: Option<usize>,
    ) -> Result<Self, std::io::Error> {
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
        let server = run(
            listener,
            pg_pool.clone(),
            email_client,
            configuration.application.base_url,
            workers,
        )?;

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

pub struct ApplicationBaseUrl(pub String);

fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
    workers: Option<usize>,
) -> Result<Server, std::io::Error> {
    let db_pool = web::Data::new(db_pool);
    let email_client = web::Data::new(email_client);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));

    let mut server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .service(health_check)
            .route("/newsletters", web::post().to(publish_newsletter))
            .route("/subscriptions", web::post().to(subscribe))
            .route("/subscriptions/confirm", web::get().to(confirm))
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
    });

    if let Some(worker_number) = workers {
        server = server.workers(worker_number);
    }

    Ok(server.listen(listener)?.run())
}
