mod common;

#[cfg(test)]
#[tokio::test]
async fn health_check_works() {
    // Arrange
    let (server, _) = common::test_setup().await;

    // Act
    let response = server.get("/health").await;

    // Assert
    response.assert_status_ok();
    response.assert_text("");
}
