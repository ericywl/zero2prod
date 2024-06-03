use axum::http::StatusCode;
use sqlx::PgPool;
use wiremock::{matchers, Mock, ResponseTemplate};
use zero2prod::domain::{SubscriptionStatus, Url};

use crate::helpers::{self, assert_is_redirect_to};

#[sqlx::test]
async fn subscribe_works_for_valid_form_data(pool: PgPool) {
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
    assert_is_redirect_to(&response, "/");
    let html_page = test_app.get_index().await.text();
    assert!(html_page.contains("Thanks for subscribing"));
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
        .fetch_one(&*test_app.app_state.db_pool)
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
async fn subscribe_returns_error_when_data_is_missing(pool: PgPool) {
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
async fn subscribe_returns_error_when_fields_are_present_but_invalid(pool: PgPool) {
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
        assert_is_redirect_to(&response, "/");
        let html_page = test_app.get_index().await.text();
        assert!(
            html_page.contains("Invalid form data"),
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
    assert_is_redirect_to(&response, "/");
    let html_page = test_app.get_index().await.text();
    assert!(html_page.contains("Thanks for subscribing"));
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
        confirmation_links.plain_text.as_str(),
        "HTML and plain text confirmation links not equal"
    );
}

#[sqlx::test]
async fn subscribe_sends_two_identical_confirmation_emails_if_called_twice(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;
    let name = "Le Mao".to_string();
    let email = "lemao@gmail.com".to_string();

    // Act
    let first_html_link: Url;
    let second_html_link: Url;

    {
        let _mock_guard = Mock::given(matchers::path("/email"))
            .and(matchers::method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount_as_scoped(&test_app.email_server)
            .await;

        let confirmation_links = test_app
            .post_subscriptions_and_extract_confirmation_link(
                Some(name.clone()),
                Some(email.clone()),
            )
            .await;
        first_html_link = confirmation_links.html.clone();
    }

    {
        let _mock_guard = Mock::given(matchers::path("/email"))
            .and(matchers::method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount_as_scoped(&test_app.email_server)
            .await;

        let confirmation_links = test_app
            .post_subscriptions_and_extract_confirmation_link(
                Some(name.clone()),
                Some(email.clone()),
            )
            .await;
        second_html_link = confirmation_links.html.clone();
    }

    assert_eq!(
        first_html_link.as_str(),
        second_html_link.as_str(),
        "HTML links from 2 separate"
    )
}

#[sqlx::test]
async fn subscribe_returns_error_if_subscription_already_confirmed(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;
    let name = "Le Mao".to_string();
    let email = "lemao@gmail.com".to_string();

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    let response = test_app
        .post_subscriptions_and_try_confirm(Some(name.clone()), Some(email.clone()))
        .await;
    response.assert_status_ok();

    // Act
    let response = test_app.post_subscriptions(Some(name), Some(email)).await;

    // Assert
    assert_is_redirect_to(&response, "/");
    let html_page = test_app.get_index().await.text();
    assert!(html_page.contains("Subscription already confirmed"));
}

#[sqlx::test]
async fn subscribe_fails_if_there_is_a_fatal_database_error(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;
    // Sabotage the database
    sqlx::query!("ALTER TABLE subscription_tokens DROP COLUMN subscription_token;",)
        .execute(&*test_app.app_state.db_pool)
        .await
        .unwrap();

    // Act
    let response = test_app
        .post_subscriptions(
            Some("le guin".into()),
            Some("ursula_le_guin@gmail.com".into()),
        )
        .await;

    // Assert
    assert_is_redirect_to(&response, "/");
    let html_page = test_app.get_index().await.text();
    assert!(html_page.contains("Something went wrong"));
}
