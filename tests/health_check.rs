//Change branche test1
use actix_hello::configuration::get_configuration;
use sqlx::{Connection, PgConnection};
use std::net::TcpListener;
use test_case::test_case;

fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let server = actix_hello::startup::run(listener).expect("Failed to bind address");
    tokio::spawn(server);

    format!("http://127.0.0.1:{}", port)
}

#[tokio::test]
async fn health_check_woeks() {
    let address = spawn_app();

    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/health_check", address))
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
    let address = spawn_app();
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{}/subscriptions", address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body.to_string())
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(status, response.status());
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let address = spawn_app();
    // let configuration = get_configuration().expect("Failed to read configuration");
    // let connection_string = configuration.database.connection_string();
    // let mut connection = PgConnection::connect(&connection_string)
    //    .await
    //    .expect("Failed to connect to Postgres");

    let client = reqwest::Client::new();
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let response = client
        .post(format!("{}/subscriptions", address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body.to_string())
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(reqwest::StatusCode::OK, response.status());

    //    let saved = sqlx::query!("SELECT email, name FROM subscriptions LIMIT 1;")
    //        .fetch_one(&mut connection)
    //        .await
    //        .expect("Failed to fetch seved subscription.");
}
