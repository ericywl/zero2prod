mod common;

use axum::http::StatusCode;

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    // Arrange
    let server = common::test_server();

    // Act
    let body = &[("name", "Bob Banjo"), ("email", "bob_banjo@gmail.com")];
    let response = server.post("/subscribe").form(body).await;

    // Assert
    assert_eq!(response.status_code(), StatusCode::OK)
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
