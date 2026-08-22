use crate::configuration::Settings;
use crate::email_client::EmailClient;
use crate::routes::subscriptions_confirm::confirm;
use crate::routes::{
    admin_dashboard, health_check, home, login, login_form, publish_newsletter, subscribe,
};
use actix_session::SessionMiddleware;
use actix_session::storage::RedisSessionStore;
use actix_web::cookie::Key;
use actix_web::{App, HttpServer, dev::Server, web};
use actix_web_flash_messages::FlashMessagesFramework;
use actix_web_flash_messages::storage::CookieMessageStore;
use anyhow::Context;
use secrecy::{ExposeSecret, SecretString};
use sqlx::PgPool;
use std::net::{SocketAddr, TcpListener};
use tracing_actix_web::TracingLogger;

pub struct Application {
    address: SocketAddr,
    server: Server,
}

impl Application {
    pub async fn build(
        configuration: Settings,
        pg_pool: &PgPool,
        workers: Option<usize>,
    ) -> Result<Self, anyhow::Error> {
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
            configuration.application.hmac_secret,
            configuration.redis_uri,
            workers,
        )
        .await?;

        Ok(Self {
            address: local_address,
            server,
        })
    }
    pub fn address(&self) -> &SocketAddr {
        &self.address
    }
    pub async fn run(self) -> Result<(), anyhow::Error> {
        self.server.await.context("Failed to run server")
    }
}

pub struct ApplicationBaseUrl(pub String);
pub struct HmacSecret(pub SecretString);

async fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
    hmac_secret: SecretString,
    redis_uri: SecretString,
    workers: Option<usize>,
) -> Result<Server, anyhow::Error> {
    let db_pool = web::Data::new(db_pool);
    let email_client = web::Data::new(email_client);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));

    let secret_key = Key::from(hmac_secret.expose_secret().as_bytes());
    let message_store = CookieMessageStore::builder(secret_key.clone()).build();
    let message_framework = FlashMessagesFramework::builder(message_store).build();
    let redis_store = RedisSessionStore::new(redis_uri.expose_secret()).await?;

    let mut server = HttpServer::new(move || {
        App::new()
            .wrap(message_framework.clone())
            .wrap(SessionMiddleware::new(
                redis_store.clone(),
                secret_key.clone(),
            ))
            .wrap(TracingLogger::default())
            .service(health_check)
            .route("/", web::get().to(home))
            .route("/login", web::get().to(login_form))
            .route("/login", web::post().to(login))
            .route("/admin/dashboard", web::get().to(admin_dashboard))
            .route("/newsletters", web::post().to(publish_newsletter))
            .route("/subscriptions", web::post().to(subscribe))
            .route("/subscriptions/confirm", web::get().to(confirm))
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
            .app_data(web::Data::new(HmacSecret(hmac_secret.clone())))
    });

    if let Some(worker_number) = workers {
        server = server.workers(worker_number);
    }

    Ok(server.listen(listener)?.run())
}
