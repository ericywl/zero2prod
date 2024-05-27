use sqlx::PgPool;

use crate::helpers;

#[sqlx::test]
async fn an_error_flash_message_is_set_on_failure(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;

    // Act - Login
    let login_body = serde_json::json!({
        "username": "some-username",
        "password": "some-password"
    });
    let response = test_app.post_login(&login_body).await;

    // Assert
    helpers::assert_is_redirect_to(&response, "/login");
    let flash_cookie = response.cookie("_flash");
    assert_eq!(flash_cookie.value(), "Authentication failed");

    // Act 2 - Follow redirect
    let html_page = test_app.get_login_html().await;
    assert!(html_page.contains("Authentication failed"));

    // Act 3 - Reload login page
    let html_page = test_app.get_login_html().await;
    assert!(!html_page.contains("Authentication failed"));
}
