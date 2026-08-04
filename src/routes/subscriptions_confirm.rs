use crate::{domain::SubscriberToken, routes::SubscriptionsStatus};
pub(crate) use actix_web::HttpResponse;
use actix_web::web;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscribtion_token: SubscriberToken,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip_all)]
pub async fn confirm(parameters: web::Query<Parameters>, pool: web::Data<PgPool>) -> HttpResponse {
    let token_data = match get_subscriber_id_from_token(&pool, &parameters.subscribtion_token).await
    {
        Ok(val) => val,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    match token_data {
        Some((subscriber_id, is_used)) => {
            if !is_used
                && confirm_subscriber(&pool, subscriber_id, &parameters.subscribtion_token)
                    .await
                    .is_err()
            {
                return HttpResponse::InternalServerError().finish();
            }
            HttpResponse::Ok().finish()
        }
        None => HttpResponse::Unauthorized().finish(),
    }
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip_all)]
async fn confirm_subscriber(
    pool: &PgPool,
    subscriber_id: Uuid,
    token: &SubscriberToken,
) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;
    sqlx::query!(
        r#" UPDATE subscriptions SET status = $1 WHERE id = $2;"#,
        SubscriptionsStatus::Confirmed as SubscriptionsStatus,
        subscriber_id,
    )
    .execute(&mut *transaction)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    sqlx::query!(
        r#"UPDATE subscription_tokens SET used_at = $1 WHERE subscription_token = $2;"#,
        Utc::now(),
        token.as_ref(),
    )
    .execute(&mut *transaction)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    transaction.commit().await.map_err(|e| {
        tracing::error!("Failed to commint transaction: {:?}", e);
        e
    })?;
    Ok(())
}

#[tracing::instrument(name = "Get subscriber_id from token", skip_all)]
async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &SubscriberToken,
) -> Result<Option<(Uuid, bool)>, sqlx::Error> {
    let result = sqlx::query!(
        r#"SELECT subscriber_id, (used_at IS NOT NULL) as "is_used!" FROM subscription_tokens WHERE subscription_token = $1"#,
        subscription_token.as_ref()
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(result.map(|r| (r.subscriber_id, r.is_used)))
}
