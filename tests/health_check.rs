use actix_hello::startup::run;
use actix_hello::telemetry::{get_subscriber, init_subscriber};
use sqlx::PgPool;
use std::net::TcpListener;
use std::sync::LazyLock;

static TRACING: LazyLock<()> = LazyLock::new(|| {
    let default_filter_level = "info";
    let subscriber_name = "test";
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    }
});

pub struct TestApp {
    pub address: String,
}

async fn spawn_app(db_pool: &PgPool) -> TestApp {
    LazyLock::force(&TRACING);
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    let server = run(listener, db_pool.clone()).expect("Failed to bind address");
    tokio::spawn(server);

    TestApp { address }
}

#[sqlx::test]
async fn health_check_woeks(pool: PgPool) {
    let test_app = spawn_app(&pool).await;

    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/health_check", test_app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[sqlx::test]
async fn subscribe_return_expected_status(pool: PgPool) {
    let test_app = spawn_app(&pool).await;
    let client = reqwest::Client::new();

    let tast_cases = vec![
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
            "400 for missing both parametrs",
        ),
    ];

    for (body, status, case) in tast_cases {
        let response = client
            .post(format!("{}/subscriptions", test_app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body.to_string())
            .send()
            .await
            .expect("Failed to execute request.");

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

    let client = reqwest::Client::new();
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let response = client
        .post(format!("{}/subscriptions", test_app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body.to_string())
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(reqwest::StatusCode::OK, response.status());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions LIMIT 1;")
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch seved subscription.");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
}

#[sqlx::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_empty(pool: PgPool) {
    let test_app = spawn_app(&pool).await;
    let client = reqwest::Client::new();

    let tast_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-email", "invalid email"),
    ];

    for (body, description) in tast_cases {
        let response = client
            .post(format!("{}/subscriptions", test_app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.");

        assert_eq!(
            reqwest::StatusCode::BAD_REQUEST,
            response.status(),
            "Got wrong status, expect {}",
            description
        );
    }
}
