use actix_web::{HttpResponse, ResponseError, http::StatusCode, web};
use anyhow::Context;
use chrono::Utc;
use rand::{RngExt, distr::Alphabetic, rngs::ThreadRng};
use serde::Deserialize;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::{
    domain::{NewSubscriber, SubscriberToken},
    email_client::EmailClient,
    startup::ApplicationBaseUrl,
};

#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

#[derive(sqlx::Type, Debug, PartialEq)]
#[sqlx(type_name = "subscriptions_status", rename_all = "snake_case")]
pub enum SubscriptionsStatus {
    PendingConfirmation,
    Confirmed,
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip_all,
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, SubscribeError> {
    let form = form.into_inner();
    let new_subscriber =
        NewSubscriber::build(form.email, form.name).map_err(SubscribeError::ValidationError)?;
    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool")?;
    let (subscriber_id, status) = insert_subscriber(&mut transaction, &new_subscriber)
        .await
        .context("Failed to insert new subscriber in the database")?;
    if status == SubscriptionsStatus::Confirmed {
        transaction
            .commit()
            .await
            .context("Failed to commin SQL transaction for existing user")?;
        return Ok(HttpResponse::Ok().finish());
    }
    let subscription_token = generate_subscription_token();
    store_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .context("Failed to store the confirmation token for new subscriber")?;
    transaction
        .commit()
        .await
        .context("Failed to commin SQL transaction for new user")?;
    send_confirmation_email(
        &email_client,
        &new_subscriber,
        &base_url.0,
        &subscription_token,
    )
    .await
    .context("Failed to send a confirmation email")?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "Sending confirmation email to a new subscriber", skip_all)]
async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: &NewSubscriber,
    base_url: &str,
    subscriptions_token: &SubscriberToken,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscribtion_token={}",
        base_url,
        subscriptions_token.as_ref()
    );
    let plain_body = format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
        confirmation_link
    );
    let html_body = format!(
        "Welcome to our newsletter! <br />
                Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );

    email_client
        .send_email(&new_subscriber.email, "Welcome", &html_body, &plain_body)
        .await
}

#[tracing::instrument(name = "Saving new subscriber details in the database", skip_all)]
async fn insert_subscriber(
    connection: &mut PgConnection,
    new_subscriber: &NewSubscriber,
) -> Result<(Uuid, SubscriptionsStatus), sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    let record = sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (email) DO UPDATE
        SET name = EXCLUDED.name
        RETURNING id, status "status: SubscriptionsStatus"
        "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now(),
        SubscriptionsStatus::PendingConfirmation as SubscriptionsStatus,
    )
    .fetch_one(connection)
    .await?;
    Ok((record.id, record.status))
}

#[tracing::instrument(name = "Store supscription token in the database", skip_all)]
async fn store_token(
    connection: &mut PgConnection,
    subscriber_id: Uuid,
    subscription_token: &SubscriberToken,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscription_token, subscriber_id) VALUES ($1, $2)"#,
        subscription_token.as_ref(),
        subscriber_id
    )
    .execute(connection)
    .await?;
    Ok(())
}

fn generate_subscription_token() -> SubscriberToken {
    let mut rng = ThreadRng::default();
    let token_str: String = std::iter::repeat_with(|| rng.sample(Alphabetic))
        .map(char::from)
        .take(25)
        .collect();
    token_str.try_into().expect("generator create wrong token!")
}

pub fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}

#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}
impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}
impl ResponseError for SubscribeError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::ValidationError(_) => StatusCode::BAD_REQUEST,
            Self::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
