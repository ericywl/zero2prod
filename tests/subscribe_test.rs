mod common;

use axum::http::StatusCode;

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    // Arrange
    let (server, state) = common::test_setup().await;
    let mut connection = state
        .db_pool
        .acquire()
        .await
        .expect("Failed to get connection from pool.");

    // Act
    let name = "Bob Banjo";
    let email = "bob_banjo@gmail.com";
    let body = &[("name", name), ("email", email)];
    let response = server.post("/subscribe").form(body).await;

    // Assert
    assert_eq!(response.status_code(), StatusCode::OK);

    let saved = sqlx::query!("SELECT name, email FROM subscriptions",)
        .fetch_one(&mut *connection)
        .await
        .expect("Failed to fetch saved subscription");
    assert_eq!(saved.name, name);
    assert_eq!(saved.email, email);
}

#[tokio::test]
async fn subscribe_returns_400_when_data_is_missing() {
    // Arrange
    let (server, _) = common::test_setup().await;
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
