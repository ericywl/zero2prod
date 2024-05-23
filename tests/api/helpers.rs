use std::sync::Arc;

use axum_test::{TestResponse, TestServer};
use once_cell::sync::Lazy;
use sqlx::PgPool;

use wiremock::MockServer;
use zero2prod::{
    configuration::get_configuration,
    domain::Url,
    startup::{default_app_state, AppState},
    telemetry::{get_subscriber, init_subscriber},
};

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    }
});

pub struct ConfirmationLinks {
    pub html: Url,
    pub plain_text: Url,
}

pub struct TestApp {
    pub server: TestServer,
    pub app_state: Arc<AppState>,
    pub email_server: MockServer,
}

impl TestApp {
    pub async fn setup(pool: PgPool) -> Self {
        Lazy::force(&TRACING);

        // Launch mock server to stand in for Postmark's API
        let email_server = MockServer::start().await;

        let config = {
            let mut c = get_configuration().expect("Failed to read configuration.");
            // Overwrite email client URL to use mock server
            c.email_client.base_url = email_server.uri();
            c
        };

        let app_state = Arc::new(default_app_state(&config, Some(pool)));
        let address = config
            .application
            .address()
            .expect("Failed to parse address.");

        let app = zero2prod::startup::Application::new(address, app_state.clone());
        let server = TestServer::new(app.router()).expect("Failed to spawn test server");

        Self {
            server,
            app_state,
            email_server,
        }
    }

    /// Send POST request to `/subscribe` with name and email.
    pub async fn post_subscriptions(
        &self,
        name: Option<String>,
        email: Option<String>,
    ) -> TestResponse {
        let mut data = vec![];
        if name.is_some() {
            data.push(("name", name.unwrap()))
        }
        if email.is_some() {
            data.push(("email", email.unwrap()))
        }

        self.server.post("/subscribe").form(&data).await
    }

    /// Send POST request to `/subscribe` with name and email.
    /// Asserts that the response for the request is OK and extracts confirmation links
    /// from the response body.
    ///
    /// Returns link extracted from HTML and plaintext body.
    pub async fn post_subscriptions_and_extract_confirmation_link(
        &self,
        name: Option<String>,
        email: Option<String>,
    ) -> ConfirmationLinks {
        let response = self.post_subscriptions(name, email).await;
        response.assert_status_ok();

        let email_request = &self.email_server.received_requests().await.unwrap()[0];
        // Parse body as JSON
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();
        // Extract link from request fields
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            // There should only be 1 link for confirmation
            assert_eq!(links.len(), 1);

            links[0].as_str().to_string()
        };

        let html_link = get_link(&body["HtmlBody"].as_str().unwrap());
        let text_link = get_link(&body["TextBody"].as_str().unwrap());

        ConfirmationLinks {
            html: Url::parse(html_link).expect("Failed to parse html confirmation link."),
            plain_text: Url::parse(text_link)
                .expect("Failed to parse plain text confirmation link."),
        }
    }
}
