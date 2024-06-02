use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use axum::http::StatusCode;
use axum_test::{TestResponse, TestServer};
use once_cell::sync::Lazy;
use sqlx::PgPool;
use uuid::Uuid;
use wiremock::MockServer;

use zero2prod::{
    configuration::get_configuration,
    domain::Url,
    startup::{default_app_state_and_session, AppState},
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

#[derive(Debug, Clone)]
pub struct TestUser {
    pub user_id: Uuid,
    pub username: String,
    pub password: String,
}

impl TestUser {
    pub fn generate() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }

    async fn store(&self, pool: &PgPool) {
        let salt = SaltString::generate(&mut rand::thread_rng());
        let password_hash = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::new(15000, 2, 1, None).unwrap(),
        )
        .hash_password(self.password.as_bytes(), &salt)
        .unwrap()
        .to_string();

        sqlx::query!(
            "INSERT INTO users (user_id, username, password_hash)
            VALUES ($1, $2, $3)",
            self.user_id,
            self.username,
            password_hash
        )
        .execute(pool)
        .await
        .expect("Failed to create test users.");
    }
}

pub struct ConfirmationLinks {
    pub html: Url,
    pub plain_text: Url,
}

pub struct TestApp {
    pub app_server: TestServer,
    pub app_state: AppState,
    pub email_server: MockServer,
    pub test_user: TestUser,
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

        let (app_state, session_layer) = default_app_state_and_session(&config, Some(pool)).await;
        let address = config
            .application
            .address()
            .expect("Failed to parse address.");

        let app = zero2prod::startup::Application::new(address, app_state.clone(), session_layer);
        let mut app_server = TestServer::new(app.router()).expect("Failed to spawn test server");
        app_server.do_save_cookies();

        // Setup test user
        let test_user = TestUser::generate();
        test_user.store(&app_state.db_pool.clone()).await;

        Self {
            app_server,
            app_state,
            email_server,
            test_user,
        }
    }

    pub async fn query_link_with_params(&self, link: &Url) -> TestResponse {
        self.app_server
            .get(link.path())
            .add_query_params(link.query_params())
            .await
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

        self.app_server.post("/subscribe").form(&data).await
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
            // There should be at least 1 link for confirmation
            assert!(links.len() > 0);
            // The link is always the last one
            links.last().unwrap().as_str().to_string()
        };

        let html_link = get_link(&body["HtmlBody"].as_str().unwrap());
        let text_link = get_link(&body["TextBody"].as_str().unwrap());

        ConfirmationLinks {
            html: Url::parse(&html_link).expect("Failed to parse html confirmation link."),
            plain_text: Url::parse(&text_link)
                .expect("Failed to parse plain text confirmation link."),
        }
    }

    pub async fn post_subscriptions_and_try_confirm(
        &self,
        name: Option<String>,
        email: Option<String>,
    ) -> TestResponse {
        let confirmation_links = self
            .post_subscriptions_and_extract_confirmation_link(name, email)
            .await;

        self.query_link_with_params(&confirmation_links.html).await
    }

    pub async fn get_login(&self) -> TestResponse {
        self.app_server.get("/login").await
    }

    pub async fn login_as_test_user(&self) -> TestResponse {
        self.post_login(&serde_json::json!({
            "username": self.test_user.username,
            "password": self.test_user.password,
        }))
        .await
    }

    pub async fn post_login<Body>(&self, body: &Body) -> TestResponse
    where
        Body: serde::Serialize,
    {
        self.app_server.post("/login").form(body).await
    }

    pub async fn post_admin_logout(&self) -> TestResponse {
        self.app_server.post("/admin/logout").await
    }

    pub async fn get_admin_dashboard(&self) -> TestResponse {
        self.app_server.get("/admin/dashboard").await
    }

    pub async fn get_admin_change_password(&self) -> TestResponse {
        self.app_server.get("/admin/password").await
    }

    pub async fn post_admin_change_password<Body>(&self, body: &Body) -> TestResponse
    where
        Body: serde::Serialize,
    {
        self.app_server.post("/admin/password").form(body).await
    }

    /// Send POST request to `/newsletters`.
    pub async fn post_newsletters(&self, body: serde_json::Value) -> TestResponse {
        self.app_server.post("/admin/newsletters").json(&body).await
    }
}

pub fn assert_is_redirect_to(response: &TestResponse, location: &str) {
    response.assert_status(StatusCode::SEE_OTHER);
    assert_eq!(response.headers().get("Location").unwrap(), location);
}
