use sqlx::PgPool;

use crate::helpers::{self, assert_is_redirect_to};

#[sqlx::test]
async fn must_be_logged_in_to_access_the_admin_dashboard(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;

    // Act
    let response = test_app.get_admin_dashboard().await;

    // Assert
    helpers::assert_is_redirect_to(&response, "/login");
}

#[sqlx::test]
async fn logout_clears_session_state(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;

    // Act & Assert 1 - Login
    let response = test_app
        .post_login(&serde_json::json!({
            "username": &test_app.test_user.username,
            "password": &test_app.test_user.password,
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    // Act & Assert 2 - Follow redirect
    let html_page = test_app.get_admin_dashboard().await.text();
    assert!(html_page.contains(&format!("Welcome {}", test_app.test_user.username)));

    // Act & Assert 3 - Logout
    let response = test_app.post_admin_logout().await;
    assert_is_redirect_to(&response, "/login");

    // Act & Assert 4 - Follow redirect
    let html_page = test_app.get_login().await.text();
    assert!(html_page.contains("You have successfully logged out"));

    // Act & Assert 5 - Attempt to load admin dashboard
    let response = test_app.get_admin_dashboard().await;
    assert_is_redirect_to(&response, "/login");
}
