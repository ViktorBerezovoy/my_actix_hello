use crate::helpers::spawn_app;
use sqlx::PgPool;

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
