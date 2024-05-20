use axum_test::TestServer;

fn test_server() -> TestServer {
    let app = zero2prod::app();
    TestServer::new(app).expect("Failed to spawn test server")
}

#[tokio::test]
async fn health_check_works() {
    // Arrange
    let server = test_server();

    // Act
    let response = server.get("/health").await;

    // Assert
    response.assert_status_ok();
    response.assert_text("");
}
