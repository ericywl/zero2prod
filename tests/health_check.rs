mod common;

#[cfg(test)]
#[tokio::test]
async fn health_check_works() {
    // Arrange
    let server = common::test_server();

    // Act
    let response = server.get("/health").await;

    // Assert
    response.assert_status_ok();
    response.assert_text("");
}
