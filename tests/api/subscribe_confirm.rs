use axum::http::StatusCode;
use sqlx::PgPool;
use wiremock::{matchers, Mock, ResponseTemplate};

use crate::helpers;
use zero2prod::domain::SubscriptionStatus;

#[sqlx::test]
async fn the_link_returned_by_subscribe_returns_ok_if_called(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;
    let name = "Adaya";
    let email = "adayayadaya@yaya.com";

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    let confirmation_links = test_app
        .post_subscriptions_and_extract_confirmation_link(Some(name.into()), Some(email.into()))
        .await;
    let html_confirmation_link = confirmation_links.html;
    // Make sure we don't accidentally call random APIs on the web
    assert_eq!(html_confirmation_link.host_str().unwrap(), "127.0.0.1");

    // Act
    let response = test_app
        .query_link_with_params(&html_confirmation_link)
        .await;

    // Assert
    response.assert_status_ok()
}

#[sqlx::test]
async fn confirmation_without_token_are_rejected_with_error(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;

    // Act
    let response = test_app.app_server.get("/subscribe/confirm").await;

    // Assert
    response.assert_status_bad_request();
}

#[sqlx::test]
async fn clicking_on_confirmation_link_confirms_a_subscriber(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;
    let name = "Adaya";
    let email = "adayayadaya@yaya.com";

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    // Act
    let response = test_app
        .post_subscriptions_and_try_confirm(Some(name.into()), Some(email.into()))
        .await;
    response.assert_status_ok();

    // Assert
    let saved = sqlx::query!("SELECT name, email, status FROM subscriptions",)
        .fetch_one(&*test_app.app_state.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.name, name, "Name not equal");
    assert_eq!(saved.email, email, "Email not equal");
    assert_eq!(
        saved.status,
        SubscriptionStatus::Confirmed.to_string(),
        "Status not confirmed"
    )
}

#[sqlx::test]
async fn confirm_returns_error_if_subscription_already_confirmed(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;
    let name = "Ayaya";
    let email = "ayaya@gachi.com";

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    let confirmation_links = test_app
        .post_subscriptions_and_extract_confirmation_link(Some(name.into()), Some(email.into()))
        .await;
    let html_confirmation_link = confirmation_links.html;
    assert_eq!(html_confirmation_link.host_str().unwrap(), "127.0.0.1");

    // Already confirmed once
    let response = test_app
        .query_link_with_params(&html_confirmation_link)
        .await;
    response.assert_status_ok();

    // Act
    let response = test_app
        .query_link_with_params(&html_confirmation_link)
        .await;

    // Assert
    response.assert_status(StatusCode::CONFLICT);
}
