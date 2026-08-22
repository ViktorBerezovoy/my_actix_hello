use actix_web::{
    HttpResponse,
    http::header::{ContentType, LOCATION},
    web,
};
use anyhow::Context;
use sqlx::PgPool;
use std::fmt::{Debug, Display};
use uuid::Uuid;

use crate::session_state::TypedSession;

fn e500<T>(e: T) -> actix_web::Error
where
    T: Debug + Display + 'static,
{
    actix_web::error::ErrorInternalServerError(e)
}

pub async fn admin_dashboard(
    session: TypedSession,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let username = if let Some(user_id) = session.get_user_id().map_err(e500)? {
        get_user(user_id, &pool).await.map_err(e500)?
    } else {
        return Ok(HttpResponse::SeeOther()
            .insert_header((LOCATION, "/login"))
            .finish());
    };
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(include_str!("dashboard.html").replace("{username}", &username)))
}

#[tracing::instrument(name = "Get username", skip_all, fields(user_id))]
async fn get_user(user_id: Uuid, pool: &PgPool) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT username
        FROM users
        WHERE user_id = $1
        "#,
        user_id,
    )
    .fetch_one(pool)
    .await
    .context("Failed to perform a query to retrive a username.")?;

    Ok(row.username)
}
