use actix_web::{HttpResponse, Responder, post, web};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;

#[allow(dead_code)]
#[derive(Deserialize)]
struct FormData {
    email: String,
    name: String,
}
#[post("/subscriptions")]
async fn subscribe(form: web::Form<FormData>, pool: web::Data<PgPool>) -> impl Responder {
    let result = sqlx::query!(r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(pool.get_ref())
    .await;

    match result {
        Ok(_) => HttpResponse::Ok(),
        Err(e) => {
            println!("Failed to execute query: {}", e);
            HttpResponse::InternalServerError()
        }
    }
}
