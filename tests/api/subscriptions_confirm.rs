use actix_hello::routes::SubscriptionsStatus;
use sqlx::PgPool;
use wiremock::{
    Mock, ResponseTemplate,
    matchers::{method, path},
};

use crate::helpers::spawn_app;

#[sqlx::test]
async fn confimations_without_token_are_rejected_with_a_400(pool: PgPool) {
    let test_app = spawn_app(&pool).await;

    let response = reqwest::get(&format!("{}/subscriptions/confirm", test_app.address))
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST)
}

#[sqlx::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called(pool: PgPool) {
    let test_app = spawn_app(&pool).await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(method("POST"))
        .and(path("/emails"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;
    test_app.post_subscriptions(body).await;
    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = test_app.get_confirmation_link(email_request);
    let response = reqwest::get(confirmation_links.html).await.unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK)
}
#[sqlx::test]
async fn clicking_on_the_confirmation_link_confirms_a_subscriber(pool: PgPool) {
    let test_app = spawn_app(&pool).await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(method("POST"))
        .and(path("/emails"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;
    test_app.post_subscriptions(body).await;
    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = test_app.get_confirmation_link(email_request);
    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let saved = sqlx::query!(
        r#"SELECT email, name, status AS "status: SubscriptionsStatus" FROM subscriptions LIMIT 1;"#
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to fetch saved subscription.");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.status, SubscriptionsStatus::Confirmed);
}
#[sqlx::test]
async fn clicking_on_the_confirmation_link_twice_returns_200(pool: PgPool) {
    let test_app = spawn_app(&pool).await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(method("POST"))
        .and(path("/emails"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;
    test_app.post_subscriptions(body).await;
    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = test_app.get_confirmation_link(email_request);
    reqwest::get(confirmation_links.html.clone())
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let response = reqwest::get(confirmation_links.html).await.unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK)
}
