mod common;

use axum::http::StatusCode;
use sqlx::{Connection, PgConnection};
use zero2prod::configuration::get_configuration;

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    // Arrange
    let server = common::test_server();
    let config = get_configuration().expect("Failed to read configuration.");
    let connection_string = config.database.connection_string();
    let mut connection = PgConnection::connect(&connection_string)
        .await
        .expect("Failed to connect to Postgres.");

    // Act
    let name = "Bob Banjo";
    let email = "bob_banjo@gmail.com";
    let body = &[("name", name.clone()), ("email", email.clone())];
    let response = server.post("/subscribe").form(body).await;

    // Assert
    assert_eq!(response.status_code(), StatusCode::OK);

    let saved = sqlx::query!("SELECT name, email FROM subscriptions",)
        .fetch_one(&mut connection)
        .await
        .expect("Failed to fetch saved subscription");
    assert_eq!(saved.name, name);
    assert_eq!(saved.email, email);
}

#[tokio::test]
async fn subscribe_returns_400_when_data_is_missing() {
    // Arrange
    let server = common::test_server();
    let test_cases = vec![
        (&[("name", "Bob Banjo")], "missing the email"),
        (&[("email", "bob_banjo@gmail.com")], "missing the name"),
        (&[("", "")], "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        // Act
        let response = server.post("/subscribe").form(invalid_body).await;

        // Assert
        assert_eq!(
            response.status_code(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "API did not fail with 400 Bad Request when payload was {}",
            error_message
        );
    }
}
