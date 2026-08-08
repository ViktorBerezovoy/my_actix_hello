use sqlx::PgPool;
use wiremock::{
    Mock, ResponseTemplate,
    matchers::{any, method, path},
};

use crate::helpers::{ConfirmationLinks, TestApp, spawn_app};

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let _mock_guard = Mock::given(method("POST"))
        .and(path("/emails"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;
    app.post_subscriptions(body)
        .await
        .error_for_status()
        .unwrap();

    let email_request = &app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();

    app.get_confirmation_link(email_request)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let confirmation_link = create_unconfirmed_subscriber(app).await;
    reqwest::get(confirmation_link.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}

#[sqlx::test]
async fn newsletter_are_not_delivered_to_uconfirmed_subscriber(db_pool: PgPool) {
    let app = spawn_app(&db_pool).await;
    create_unconfirmed_subscriber(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        }
    });
    let response = app.post_newsletters(&newsletter_request_body).await;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
}

#[sqlx::test]
async fn newsletter_are_delivered_to_confirmed_subscribers(pool: PgPool) {
    let app = spawn_app(&pool).await;
    create_confirmed_subscriber(&app).await;

    Mock::given(method("POST"))
        .and(path("/emails"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;
    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        }
    });
    let response = app.post_newsletters(&newsletter_request_body).await;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
}

#[sqlx::test]
async fn newsletter_returns_400_for_invalid_data(pool: PgPool) {
    let app = spawn_app(&pool).await;
    let test_case = vec![
        (
            serde_json::json!({
                "content": {
                    "text": "Newsletter body as plain text",
                    "html": "<p>Newsletter body as HTML</p>",
                }
            }),
            "missing title",
        ),
        (
            serde_json::json!({"title": "Newsletter!"}),
            "missing content",
        ),
    ];

    for (invalid_body, error_message) in test_case {
        let response = app.post_newsletters(&invalid_body).await;

        assert_eq!(
            response.status(),
            reqwest::StatusCode::BAD_REQUEST,
            "The API did not fail with 400 Bad Reqwest when the payload was {}.",
            error_message
        );
    }
}
