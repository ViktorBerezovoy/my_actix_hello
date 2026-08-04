use actix_web::{HttpResponse, web};
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
) -> HttpResponse {
    let form = form.into_inner();
    let new_subscriber = match NewSubscriber::build(form.email, form.name) {
        Ok(subscriber) => subscriber,
        Err(msg) => return HttpResponse::BadRequest().body(msg),
    };
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(e) => {
            tracing::error!("Cteate transaction error: {:?}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };
    let (subscriber_id, status) = match insert_subscriber(&mut transaction, &new_subscriber).await {
        Ok(val) => val,
        Err(e) => {
            tracing::error!("Databese record error: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };
    if status == SubscriptionsStatus::Confirmed {
        if transaction.commit().await.is_err() {
            return HttpResponse::InternalServerError().finish();
        }
        return HttpResponse::Ok().finish();
    }
    let subscription_token = generate_subscription_token();
    if let Err(e) = store_token(&mut transaction, subscriber_id, &subscription_token).await {
        tracing::error!("Saving email error: {}", e);
        return HttpResponse::InternalServerError().finish();
    }
    if let Err(e) = transaction.commit().await {
        tracing::error!("Failed to commint transaction: {:?}", e);
        return HttpResponse::InternalServerError().finish();
    }
    if let Err(e) = send_confirmation_email(
        &email_client,
        &new_subscriber,
        &base_url.0,
        &subscription_token,
    )
    .await
    {
        tracing::error!("Sending email error: {}", e);
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().finish()
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
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
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
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
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
