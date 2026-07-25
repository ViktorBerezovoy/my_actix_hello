use actix_hello::configuration::{self, get_configuration};
use sqlx::{Connection, PgConnection, PgPool};
use std::net::TcpListener;
use test_case::test_case;
use actix_hello::startup::run;

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

async fn spawn_app() -> TestApp {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    let configuration = get_configuration().expect("Failed to read configuration.");
    let connection_pool = PgPool::connect(&configuration.database.connection_string())
        .await
        .expect("Failed to connect to Postgres.");

    let server = run(listener, connection_pool.clone()).expect("Failed to bind address");
    tokio::spawn(server);

    TestApp {
        address, db_pool: connection_pool,
    }
}

#[tokio::test]
async fn health_check_woeks() {
    let test_app = spawn_app().await;

    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/health_check", test_app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[test_case( "name=le%20guin", reqwest::StatusCode::BAD_REQUEST; "400 for missing email")]
#[test_case( "email=ursula_le_guin%40gmail.com", reqwest::StatusCode::BAD_REQUEST; "400 for missing name")]
#[test_case("", reqwest::StatusCode::BAD_REQUEST; "400 for missing both parametrs")]
#[tokio::test]
async fn subscribe_return_expected_status(body: &str, status: reqwest::StatusCode) {
    let test_app = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{}/subscriptions", test_app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body.to_string())
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(status, response.status());
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let test_app = spawn_app().await;

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
           .fetch_one(&test_app.db_pool)
           .await
           .expect("Failed to fetch seved subscription.");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
}
