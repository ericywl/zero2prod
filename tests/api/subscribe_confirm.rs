use reqwest::Url;
use sqlx::PgPool;
use wiremock::{matchers, Mock, ResponseTemplate};

use crate::helpers;

#[sqlx::test]
async fn the_link_returned_by_subscribe_returns_200_if_called(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;
    let name = "Adaya";
    let email = "adayayadaya@yaya.com";

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    let (html_link, _) = test_app
        .post_subscriptions_and_extract_confirmation_link(Some(name.into()), Some(email.into()))
        .await;
    let confirmation_link = Url::parse(&html_link).unwrap();
    // Make sure we don't accidentally call random APIs on the web
    assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
    let confirmation_link_path = confirmation_link.path();

    // Act
    let response = test_app.server.get(confirmation_link_path).await;

    // Assert
    response.assert_status_ok()
}

#[sqlx::test]
async fn confirmation_without_token_are_rejected_with_400(pool: PgPool) {
    // Arrange
    let test_app = helpers::TestApp::setup(pool).await;

    // Act
    let response = test_app.server.get("/subscribe/confirm").await;

    // Assert
    response.assert_status_bad_request();
}
