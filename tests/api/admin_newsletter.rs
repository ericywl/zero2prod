use axum::http::StatusCode;
use sqlx::PgPool;
use wiremock::{matchers, Mock, ResponseTemplate};

use crate::helpers::{self, assert_is_redirect_to};

fn sample_newsletter_request_body() -> impl serde::Serialize {
    serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
        "idempotency_key": uuid::Uuid::new_v4().to_string(),
    })
}

#[sqlx::test]
async fn unauthorized_requests_are_redirected_to_login(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;

    // Act
    let response = test_app
        .post_admin_newsletters(&sample_newsletter_request_body())
        .await;

    // Assert
    assert_is_redirect_to(&response, "/login");
}

/// Use the public API of the application under test to create a subscriber.
async fn create_subscriber(test_app: &helpers::TestApp, confirm: bool) {
    // Scoped mock to assert that subscription will send confirmation email
    let _mock_guard = Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&test_app.email_server)
        .await;

    let name = Some("naruto".into());
    let email = Some("naruto@gmail.come".into());

    // Whether to confirm the subscriber or not
    if confirm {
        let response = test_app
            .post_subscriptions_and_try_confirm(name, email)
            .await;
        response.assert_status_ok();
    } else {
        let _ = test_app
            .post_subscriptions_and_extract_confirmation_link(name, email)
            .await;
    }
}

#[sqlx::test]
async fn newsletters_returns_error_for_invalid_data(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;
    // Login to post newsletters
    test_app.login_as_test_user().await;

    let test_cases = vec![
        (
            serde_json::json!({
                "text_content": "Newsletter body as plain text",
                "html_content": "<p>Newsletter body as HTML</p>",
                "idempotency_key": uuid::Uuid::new_v4().to_string(),
            }),
            "missing title",
        ),
        (
            serde_json::json!({
                "title": "Newsletter!",
                "idempotency_key": uuid::Uuid::new_v4().to_string(),
            }),
            "missing content",
        ),
    ];

    for (invalid_body, error_message) in test_cases {
        // Act
        let response = test_app.post_admin_newsletters(&invalid_body).await;

        // Assert
        assert_eq!(
            StatusCode::UNPROCESSABLE_ENTITY,
            response.status_code(),
            "The API did not fail when the payload was {}.",
            error_message
        );
    }
}

#[sqlx::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;
    test_app.login_as_test_user().await;
    create_subscriber(&test_app, false).await;

    Mock::given(matchers::any())
        .respond_with(ResponseTemplate::new(200))
        // We assert that no request is fired at Postmark!
        .expect(0)
        .mount(&test_app.email_server)
        .await;

    // Act
    let response = test_app
        .post_admin_newsletters(&sample_newsletter_request_body())
        .await;

    // Assert
    assert_is_redirect_to(&response, "/admin/newsletters");
    let html_page = test_app.get_admin_newsletters().await.text();
    assert!(html_page.contains("Newsletter successfully published"));
}

#[sqlx::test]
async fn newsletters_are_delivered_to_confirmed_subscribers(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;
    test_app.login_as_test_user().await;
    create_subscriber(&test_app, true).await;

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        // We assert that 1 request is fired at Postmark!
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    // Act
    let response = test_app
        .post_admin_newsletters(&sample_newsletter_request_body())
        .await;

    // Assert
    assert_is_redirect_to(&response, "/admin/newsletters");
    let html_page = test_app.get_admin_newsletters().await.text();
    assert!(html_page.contains("Newsletter successfully published"));
    // Mock verifies on Drop that we have sent the newsletter email
}

#[sqlx::test]
async fn newsletter_creation_is_idempotent(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;
    test_app.login_as_test_user().await;
    create_subscriber(&test_app, true).await;

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    // Act & Assert 1 - Submit newsletter
    let request_body = sample_newsletter_request_body();
    let response = test_app.post_admin_newsletters(&request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");
    let html_page = test_app.get_admin_newsletters().await.text();
    assert!(html_page.contains("Newsletter successfully published"));

    // Act & Assert 2 - Submit newsletter again
    let response = test_app.post_admin_newsletters(&request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");
    let html_page = test_app.get_admin_newsletters().await.text();
    assert!(html_page.contains("Newsletter successfully published"));

    // Mock verifies on Drop that we have sent the newsletter email **once**
}

#[sqlx::test]
async fn concurrent_form_submission_is_handled_gracefully(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;
    test_app.login_as_test_user().await;
    create_subscriber(&test_app, true).await;

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        // Setting a long delay to ensure that the second request
        // arrives before the first one completes
        .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(2)))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    // Act - Submit two newsletter forms concurrently
    let newsletter_request_body = sample_newsletter_request_body();
    let response1 = test_app.post_admin_newsletters(&newsletter_request_body);
    let response2 = test_app.post_admin_newsletters(&newsletter_request_body);
    let (response1, response2) = tokio::join!(response1, response2);

    assert_eq!(response1.status_code(), response2.status_code());
    assert_eq!(response1.text(), response2.text());
    // Check that the latest call to newsletters is also successful
    assert_is_redirect_to(&response2, "/admin/newsletters");
    let html_page = test_app.get_admin_newsletters().await.text();
    assert!(html_page.contains("Newsletter successfully published"));
    // Mock verifies on Drop that we have sent the newsletter email **once**
}
