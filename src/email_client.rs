use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;
use thiserror::Error;

use crate::{
    configuration::EmailClientSettings,
    domain::{Email, ParseEmailError, ParseUrlError, Url},
};

pub struct EmailClient {
    sender: Email,
    http_client: Client,
    base_url: Url,
    authorization_token: SecretString,
    timeout: std::time::Duration,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}

#[derive(Debug, Error)]
pub enum SendEmailError {
    #[error(transparent)]
    Request(#[from] reqwest::Error),
}

impl EmailClient {
    pub fn new(
        base_url: Url,
        sender: Email,
        authorization_token: SecretString,
        timeout: std::time::Duration,
    ) -> Self {
        Self {
            http_client: Client::new(),
            base_url,
            sender,
            authorization_token,
            timeout,
        }
    }

    pub async fn send_email(
        &self,
        recipient: &Email,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), SendEmailError> {
        let url = self.base_url.join("email").unwrap(); // safely unwrap since it's proper url
        let request_body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject,
            html_body: html_content,
            text_body: text_content,
        };

        let _ = self
            .http_client
            .post(url.to_string())
            // Add Postmark token
            .header(
                "X-Postmark-Server-Token",
                self.authorization_token.expose_secret(),
            )
            .json(&request_body)
            .timeout(self.timeout)
            .send()
            .await?
            // Return error status code
            .error_for_status()?;

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum EmailClientError {
    #[error(transparent)]
    ParseEmail(#[from] ParseEmailError),

    #[error(transparent)]
    ParseUrl(#[from] ParseUrlError),
}

impl TryFrom<EmailClientSettings> for EmailClient {
    type Error = EmailClientError;

    fn try_from(settings: EmailClientSettings) -> Result<Self, Self::Error> {
        let sender = settings.sender()?;
        let base_url = settings.url()?;
        let timeout = settings.timeout();

        Ok(EmailClient::new(
            base_url,
            sender,
            settings.authorization_token,
            timeout,
        ))
    }
}

#[cfg(test)]
mod test {
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    use secrecy::Secret;
    use wiremock::matchers;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::domain::{Email, Url};
    use crate::email_client::{EmailClient, SendEmailError};

    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &wiremock::Request) -> bool {
            // Try to parse body as JSON value
            let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);
            if let Ok(body) = result {
                body.get("From").is_some()
                    && body.get("To").is_some()
                    && body.get("Subject").is_some()
                    && body.get("HtmlBody").is_some()
                    && body.get("TextBody").is_some()
            } else {
                false
            }
        }
    }

    async fn test_send_email_with_mock(mock_server: &MockServer) -> Result<(), SendEmailError> {
        let sender = Email::parse(SafeEmail().fake()).unwrap();
        let base_url = Url::parse(mock_server.uri()).unwrap();
        // Initialize email client
        let email_client = EmailClient::new(
            base_url,
            sender,
            Secret::new(Faker.fake()),
            std::time::Duration::from_millis(200),
        );

        // Generate random data
        let subscriber_email = Email::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();

        let outcome = email_client
            .send_email(&subscriber_email, &subject, &content, &content)
            .await;

        outcome
    }

    #[tokio::test]
    async fn send_email_fires_request_to_base_url() {
        // Arrange
        let mock_server = MockServer::start().await;
        Mock::given(matchers::header_exists("X-Postmark-Server-Token"))
            .and(matchers::header("Content-Type", "application/json"))
            .and(matchers::path("/email"))
            .and(matchers::method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let _ = test_send_email_with_mock(&mock_server).await;

        // Assert
        // Mock expectations are checked on drop
    }

    #[tokio::test]
    async fn send_email_succeeds_if_server_returns_200() {
        // Arrange
        let mock_server = MockServer::start().await;
        // We do not copy in all the matchers we have in the other test.
        // The purpose of this test is not to assert on the request we
        // are sending out!
        // We add the bare minimum needed to trigger the path we want
        // to test in `send_email`.
        Mock::given(matchers::any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let outcome = test_send_email_with_mock(&mock_server).await;

        // Assert
        assert!(outcome.is_ok());
    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500() {
        // Arrange
        let mock_server = MockServer::start().await;
        Mock::given(matchers::any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let outcome = test_send_email_with_mock(&mock_server).await;

        // Assert
        assert!(outcome.is_err());
    }

    #[tokio::test]
    async fn send_email_times_out_if_server_takes_too_long() {
        // Arrange
        let mock_server = MockServer::start().await;
        let responder = ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(60));
        Mock::given(matchers::any())
            .respond_with(responder)
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let outcome = test_send_email_with_mock(&mock_server).await;

        // Assert
        assert!(outcome.is_err());
    }
}
