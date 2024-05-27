use axum::http::StatusCode;
use sqlx::PgPool;
use wiremock::{matchers, Mock, ResponseTemplate};

use crate::helpers::{self, TestUser};

#[sqlx::test]
async fn requests_missing_authorization_are_rejected(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;

    // Act
    let response = test_app
        .post_newsletters_with_user(
            serde_json::json!({
                "title": "Newsletter title",
                "content": {
                    "text": "Newsletter body as plain text",
                    "html": "<p>Newsletter body as HTML</p>",
                }
            }),
            None,
        )
        .await;

    // Assert
    response.assert_status(StatusCode::UNAUTHORIZED);
    assert_eq!(
        r#"Basic realm="publish""#,
        response.headers()["WWW-Authenticate"]
    );
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
        test_app
            .post_subscriptions_and_try_confirm(name, email)
            .await;
    } else {
        let response = test_app.post_subscriptions(name, email).await;
        response.assert_status_ok();
    }
}

#[sqlx::test]
async fn newsletters_returns_error_for_invalid_data(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;
    let test_cases = vec![
        (
            serde_json::json!({
                "content": {
                    "text": "Newsletter body as plain text",
                    "html": "<p>Newsletter body as HTML</p>",
                }
            }),
            "missing title",
        ),
        (
            serde_json::json!({"title": "Newsletter!"}),
            "missing content",
        ),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = test_app
            .post_newsletters_with_default_user(invalid_body)
            .await;

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
    create_subscriber(&test_app, false).await;

    Mock::given(matchers::any())
        .respond_with(ResponseTemplate::new(200))
        // We assert that no request is fired at Postmark!
        .expect(0)
        .mount(&test_app.email_server)
        .await;

    // Act
    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        }
    });
    let response = test_app
        .post_newsletters_with_default_user(newsletter_request_body)
        .await;

    // Assert
    response.assert_status_ok();
}

#[sqlx::test]
async fn newsletters_are_delivered_to_confirmed_subscribers(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;
    create_subscriber(&test_app, true).await;

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        // We assert that 1 request is fired at Postmark!
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    // Act
    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        }
    });
    let response = test_app
        .post_newsletters_with_default_user(newsletter_request_body)
        .await;

    // Assert
    response.assert_status_ok();
    // Mock verifies on Drop that we have sent the newsletter email
}

#[sqlx::test]
async fn non_existing_user_is_rejected(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;
    // Random credentials
    let random_user = TestUser::generate();

    // Act
    let response = test_app
        .post_newsletters_with_user(
            serde_json::json!({
            "title": "Newsletter title",
            "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
            }
            }),
            Some(random_user),
        )
        .await;

    // Assert
    response.assert_status(StatusCode::UNAUTHORIZED);
    assert_eq!(
        r#"Basic realm="publish""#,
        response.headers()["WWW-Authenticate"]
    );
}

#[sqlx::test]
async fn invalid_password_is_rejected(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;
    // Random credentials, but replace username with default test username
    let mut invalid_password_user = TestUser::generate();
    invalid_password_user.username = test_app.test_user.username.clone();
    // Sanity check
    assert_ne!(invalid_password_user.password, test_app.test_user.password);

    // Act
    let response = test_app
        .post_newsletters_with_user(
            serde_json::json!({
            "title": "Newsletter title",
            "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
            }
            }),
            Some(invalid_password_user),
        )
        .await;

    // Assert
    response.assert_status(StatusCode::UNAUTHORIZED);
    assert_eq!(
        r#"Basic realm="publish""#,
        response.headers()["WWW-Authenticate"]
    );
}
