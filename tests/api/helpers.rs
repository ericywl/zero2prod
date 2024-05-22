use std::sync::Arc;

use axum_test::{TestResponse, TestServer};
use once_cell::sync::Lazy;
use sqlx::PgPool;

use wiremock::MockServer;
use zero2prod::{
    configuration::get_configuration,
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
}
