mod common;

use axum::http::StatusCode;
use axum_test::multipart::MultipartForm;

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    // Arrange
    let server = common::test_server();

    // Act
    let body = MultipartForm::new()
        .add_text("name", "Bob Banjo")
        .add_text("email", "bob_banjo@gmail.com");
    let response = server.post("/subscribe").multipart(body).await;

    // Assert
    assert_eq!(response.status_code(), StatusCode::OK)
}

#[tokio::test]
async fn subscribe_returns_400_when_data_is_missing() {
    // Arrange
    let server = common::test_server();
    let test_cases = vec![
        (
            MultipartForm::new().add_text("name", "Bob Banjo"),
            "missing the email",
        ),
        (
            MultipartForm::new().add_text("email", "bob_banjo@gmail.com"),
            "missing the name",
        ),
        (MultipartForm::new(), "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        // Act
        let response = server.post("/subscribe").multipart(invalid_body).await;

        // Assert
        assert_eq!(
            response.status_code(),
            StatusCode::BAD_REQUEST,
            "API did not fail with 400 Bad Request when payload was {}",
            error_message
        );
    }
}
