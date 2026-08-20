use actix_web::{HttpResponse, http::header::ContentType, web};
use hmac::{Hmac, KeyInit, Mac};
use secrecy::ExposeSecret;

use crate::startup::HmacSecret;

#[derive(serde::Deserialize)]
pub struct QueryParams {
    error: String,
    tag: String,
}
impl QueryParams {
    fn varify(self, secret: &HmacSecret) -> Result<String, anyhow::Error> {
        let tag = hex::decode(self.tag)?;
        let query_string = format!(
            "error={}",
            url::form_urlencoded::byte_serialize(self.error.to_string().as_bytes())
                .collect::<String>()
        );

        let mut mac =
            Hmac::<sha2::Sha256>::new_from_slice(secret.0.expose_secret().as_bytes()).unwrap();
        mac.update(query_string.as_bytes());
        mac.verify_slice(&tag)?;

        Ok(self.error)
    }
}

pub async fn login_form(
    query: Option<web::Query<QueryParams>>,
    secret: web::Data<HmacSecret>,
) -> HttpResponse {
    let error_html = match query {
        Some(query) => match query.0.varify(&secret) {
            Ok(error) => {
                format!("<p><i>{}</i></p>", html_escape::encode_text_minimal(&error))
            }
            Err(e) => {
                tracing::warn!(
                    error.message = %e,
                    error.cause_chain = ?e,
                    "Failed to verify query parametrs useing the HMAC tag"
                );
                "".into()
            }
        },
        None => "".into(),
    };

    let html_body = include_str!("login.html").replace("{error_html}", &error_html);
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(html_body)
}
