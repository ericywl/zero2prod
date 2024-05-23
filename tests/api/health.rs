use sqlx::PgPool;

#[sqlx::test]
async fn health_check_works(pool: PgPool) {
    // Arrange

    use crate::helpers;
    let test_app = helpers::TestApp::setup(pool).await;

    // Act
    let response = test_app.server.get("/health").await;

    // Assert
    response.assert_status_ok();
    response.assert_text("");
}
