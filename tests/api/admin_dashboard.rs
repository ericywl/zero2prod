use sqlx::PgPool;

use crate::helpers;

#[sqlx::test]
async fn must_be_logged_in_to_access_the_admin_dashboard(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;

    // Act
    let response = test_app.get_admin_dashboard().await;

    // Assert
    helpers::assert_is_redirect_to(&response, "/login");
}
