use crate::helpers::spawn_app;
use actix_hello::routes::SubscriptionsStatus;
use sqlx::PgPool;
use wiremock::{
    Mock, ResponseTemplate,
    matchers::{method, path},
};

#[sqlx::test]
async fn subscribe_return_expected_status(pool: PgPool) {
    let test_app = spawn_app(&pool).await;
    let test_cases = vec![
        (
            "name=le%20guin",
            reqwest::StatusCode::BAD_REQUEST,
            "400 for missing email",
        ),
        (
            "email=ursula_le_guin%40gmail.com",
            reqwest::StatusCode::BAD_REQUEST,
            "400 for missing name",
        ),
        (
            "",
            reqwest::StatusCode::BAD_REQUEST,
            "400 for missing both parameters",
        ),
    ];

    for (body, status, case) in test_cases {
        let response = test_app.post_subscriptions(body).await;

        assert_eq!(
            status,
            response.status(),
            "Got wrong status, expect {}",
            case
        );
    }
}

#[sqlx::test]
async fn subscribe_returns_a_200_for_valid_form_data(pool: PgPool) {
    let test_app = spawn_app(&pool).await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(method("POST"))
        .and(path("/emails"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let response = test_app.post_subscriptions(body).await;

    assert_eq!(reqwest::StatusCode::OK, response.status());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions LIMIT 1;")
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch saved subscription.");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
}

#[sqlx::test]
async fn subscribe_persists_the_new_subscriber(pool: PgPool) {
    let test_app = spawn_app(&pool).await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(method("POST"))
        .and(path("/emails"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscriptions(body).await;

    let saved = sqlx::query!(
        r#"SELECT email, name, status AS "status: SubscriptionsStatus" FROM subscriptions LIMIT 1;"#
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to fetch saved subscription.");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.status, SubscriptionsStatus::PendingConfirmation);
}

#[sqlx::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_empty(pool: PgPool) {
    let test_app = spawn_app(&pool).await;
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-email", "invalid email"),
    ];

    for (body, description) in test_cases {
        let response = test_app.post_subscriptions(body).await;

        assert_eq!(
            reqwest::StatusCode::BAD_REQUEST,
            response.status(),
            "Got wrong status, expect {}",
            description
        );
    }
}
#[sqlx::test]
async fn sebscribe_send_a_confirmation_email_for_valid_data(pool: PgPool) {
    let test_app = spawn_app(&pool).await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(method("POST"))
        .and(path("/emails"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscriptions(body).await;
}
#[sqlx::test]
async fn sebscribe_send_a_confirmation_email_with_a_link(pool: PgPool) {
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

    assert_eq!(confirmation_links.html, confirmation_links.plain_text);
}
#[sqlx::test]
async fn sebscribe_send_a_second_email_for_not_confirmed_user(pool: PgPool) {
    let test_app = spawn_app(&pool).await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(method("POST"))
        .and(path("/emails"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    let response = test_app.post_subscriptions(body).await;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let response = test_app.post_subscriptions(body).await;
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let email_requests = &test_app.email_server.received_requests().await.unwrap();

    assert_eq!(email_requests.len(), 2);
}

#[sqlx::test]
async fn subscribe_falis_if_there_is_a_fatal_database_error(pool: PgPool) {
    let test_app = spawn_app(&pool).await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    sqlx::query!("ALTER TABLE subscription_tokens DROP COLUMN subscription_token;")
        .execute(&pool)
        .await
        .unwrap();

    let response = test_app.post_subscriptions(body).await;

    assert_eq!(
        response.status(),
        reqwest::StatusCode::INTERNAL_SERVER_ERROR
    );
}
