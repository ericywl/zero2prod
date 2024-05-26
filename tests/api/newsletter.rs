use sqlx::PgPool;
use wiremock::{matchers, Mock, ResponseTemplate};

use crate::helpers;

#[sqlx::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;
    create_unconfirmed_subscriber(&test_app).await;

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
        .server
        .post("/newsletters")
        .json(&newsletter_request_body)
        .await;

    // Assert
    response.assert_status_ok();
}

/// Use the public API of the application under test to create an unconfirmed subscriber.
async fn create_unconfirmed_subscriber(test_app: &helpers::TestApp) {
    // Scoped mock to assert that subscription will send confirmation email
    let _mock_guard = Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&test_app.email_server)
        .await;

    test_app
        .post_subscriptions(Some("naruto".into()), Some("naruto@gmail.come".into()))
        .await
        .assert_status_ok();
}
