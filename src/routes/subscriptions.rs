use actix_web::{HttpResponse, Responder, post, web};
use serde::Deserialize;

#[allow(dead_code)]
#[derive(Deserialize)]
struct FormData {
    email: String,
    name: String,
}
#[post("/subscriptions")]
async fn subscribe(_req_body: web::Form<FormData>) -> impl Responder {
    HttpResponse::Ok()
}
