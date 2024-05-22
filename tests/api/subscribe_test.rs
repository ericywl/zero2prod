use axum::http::StatusCode;
use sqlx::PgPool;
use wiremock::{matchers, Mock, ResponseTemplate};

use crate::helpers;

#[cfg(test)]
#[sqlx::test]
async fn subscribe_returns_200_for_valid_form_data(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;
    let mut connection = test_app
        .app_state
        .db_pool
        .acquire()
        .await
        .expect("Failed to get connection from pool.");

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    // Act
    let name = "Bob Banjo";
    let email = "bob_banjo@gmail.com";
    let response = test_app
        .post_subscriptions(Some(name.into()), Some(email.into()))
        .await;

    // Assert
    response.assert_status_ok();

    let saved = sqlx::query!("SELECT name, email FROM subscriptions",)
        .fetch_one(&mut *connection)
        .await
        .expect("Failed to fetch saved subscription");
    assert_eq!(saved.name, name);
    assert_eq!(saved.email, email);
}

#[cfg(test)]
#[sqlx::test]
async fn subscribe_returns_400_when_data_is_missing(pool: PgPool) {
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

#[cfg(test)]
#[sqlx::test]
async fn subscribe_returns_400_when_fields_are_present_but_invalid(pool: PgPool) {
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

#[cfg(test)]
#[sqlx::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    // Act
    let response = test_app
        .post_subscriptions(Some("Le Mao".into()), Some("lemao@gmail.com".into()))
        .await;

    // Assert
    response.assert_status_ok();
}
