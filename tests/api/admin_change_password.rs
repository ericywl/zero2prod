use sqlx::PgPool;
use uuid::Uuid;

use crate::helpers::{self, assert_is_redirect_to};

#[sqlx::test]
async fn must_be_logged_in_to_see_change_password_form(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;

    // Act
    let response = test_app.get_admin_change_password().await;

    // Assert
    assert_is_redirect_to(&response, "/login")
}

#[sqlx::test]
async fn must_be_logged_in_to_change_password(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;
    let new_password = Uuid::new_v4().to_string();

    // Act
    let response = test_app
        .post_admin_change_password(&serde_json::json!({
            "current_password": Uuid::new_v4().to_string(),
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;

    // Assert
    assert_is_redirect_to(&response, "/login");
}

#[sqlx::test]
async fn new_password_fields_must_match(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;
    let new_password = Uuid::new_v4().to_string();
    let another_new_password = Uuid::new_v4().to_string();
    // Login
    test_app
        .post_login(&serde_json::json!({
            "username": &test_app.test_user.username,
            "password": &test_app.test_user.password
        }))
        .await;

    // Act
    let response = test_app
        .post_admin_change_password(&serde_json::json!({
            "current_password": Uuid::new_v4().to_string(),
            "new_password": &new_password,
            "new_password_check": &another_new_password,
        }))
        .await;

    // Assert
    assert_is_redirect_to(&response, "/admin/password");
    let html_page = test_app.get_admin_change_password().await.text();
    assert!(html_page.contains("You entered two different new passwords"));
}

#[sqlx::test]
async fn current_password_must_be_valid(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;
    let new_password = Uuid::new_v4().to_string();
    let wrong_password = Uuid::new_v4().to_string();
    // Login
    test_app
        .post_login(&serde_json::json!({
            "username": &test_app.test_user.username,
            "password": &test_app.test_user.password
        }))
        .await;

    // Act
    let response = test_app
        .post_admin_change_password(&serde_json::json!({
            "current_password": &wrong_password,
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;

    // Assert
    assert_is_redirect_to(&response, "/admin/password");
    let html_page = test_app.get_admin_change_password().await.text();
    assert!(html_page.contains("The current password is incorrect"));
}
