use axum::http::StatusCode;
use sqlx::PgPool;
use wiremock::{matchers, Mock, ResponseTemplate};
use zero2prod::domain::SubscriptionStatus;

use crate::helpers;

#[sqlx::test]
async fn subscribe_returns_200_for_valid_form_data(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;
    let name = "Bob Banjo";
    let email = "bob_banjo@gmail.com";

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    // Act
    let response = test_app
        .post_subscriptions(Some(name.into()), Some(email.into()))
        .await;

    // Assert
    response.assert_status_ok();
}

#[sqlx::test]
async fn subscribe_persists_new_subscriber(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;
    let name = "Naruto";
    let email = "naruto@konoha.co.jp";

    // Act
    test_app
        .post_subscriptions(Some(name.into()), Some(email.into()))
        .await;

    // Assert
    let saved = sqlx::query!("SELECT name, email, status FROM subscriptions",)
        .fetch_one(&test_app.app_state.db_pool)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.name, name, "Name not equal");
    assert_eq!(saved.email, email, "Email not equal");
    assert_eq!(
        saved.status,
        SubscriptionStatus::PendingConfirmation.to_string(),
        "Status not pending confirmation"
    )
}

#[sqlx::test]
async fn subscribe_returns_422_when_data_is_missing(pool: PgPool) {
    struct TestCase {
        name: Option<String>,
        email: Option<String>,
    }

    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;
    let test_cases: Vec<(&str, TestCase)> = vec![
        (
            "missing the email",
            TestCase {
                name: Some("Bob Banjo".into()),
                email: None,
            },
        ),
        (
            "missing the name",
            TestCase {
                name: None,
                email: Some("bob_banjo@gmail.com".into()),
            },
        ),
        (
            "missing both name and email",
            TestCase {
                name: None,
                email: None,
            },
        ),
    ];

    for (error_message, t) in test_cases {
        // Act
        let response = test_app.post_subscriptions(t.name, t.email).await;

        // Assert
        assert_eq!(
            response.status_code(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "API did not fail when payload was {}",
            error_message
        );
    }
}

#[sqlx::test]
async fn subscribe_returns_422_when_fields_are_present_but_invalid(pool: PgPool) {
    struct TestCase {
        name: Option<String>,
        email: Option<String>,
    }

    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;
    let test_cases: Vec<(&str, TestCase)> = vec![
        (
            "empty name",
            TestCase {
                name: Some("".into()),
                email: Some("booboo@yahoo.com".into()),
            },
        ),
        (
            "empty email",
            TestCase {
                name: Some("Aloha".into()),
                email: Some("".into()),
            },
        ),
        (
            "invalid email",
            TestCase {
                name: Some("Totally not Fake".into()),
                email: Some("definitely-not-email".into()),
            },
        ),
    ];

    for (error_message, t) in test_cases {
        // Act
        let response = test_app.post_subscriptions(t.name, t.email).await;

        // Assert
        assert_eq!(
            response.status_code(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "API did not fail when payload was {}",
            error_message
        );
    }
}

#[sqlx::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;
    let name = "Le Mao".to_string();
    let email = "lemao@gmail.com".to_string();

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    // Act
    let response = test_app.post_subscriptions(Some(name), Some(email)).await;

    // Assert
    response.assert_status_ok();
}

#[sqlx::test]
async fn subscribe_sends_confirmation_email_with_link(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;
    let name = "Le Mao".to_string();
    let email = "lemao@gmail.com".to_string();

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        // We are not setting an expectation here anymore
        // The test is focused on another aspect of the app
        // behaviour.
        .mount(&test_app.email_server)
        .await;

    // Act
    let confirmation_links = test_app
        .post_subscriptions_and_extract_confirmation_link(Some(name), Some(email))
        .await;

    // Assert
    assert_eq!(
        confirmation_links.html.as_str(),
        confirmation_links.plain_text.as_ref(),
        "HTML and plain text confirmation links not equal"
    );
}
