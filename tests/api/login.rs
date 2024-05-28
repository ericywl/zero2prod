use sqlx::PgPool;

use crate::helpers;

#[sqlx::test]
async fn an_error_flash_message_is_set_on_failure(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;

    // Act & Assert 1 - Login
    let login_body = serde_json::json!({
        "username": "some-username",
        "password": "some-password"
    });
    let response = test_app.post_login(&login_body).await;
    helpers::assert_is_redirect_to(&response, "/login");

    // Act & Assert 2 - Follow redirect
    let html_page = test_app.get_login_html().await;
    assert!(html_page.contains("Authentication failed"));

    // Act & Assert 3 - Reload login page
    let html_page = test_app.get_login_html().await;
    assert!(!html_page.contains("Authentication failed"));
}
