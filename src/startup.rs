use std::{net::SocketAddr, sync::Arc};

use super::routes;
use axum::{http::Request, routing, Router};
use sqlx::PgPool;
use tower_http::{
    trace::{DefaultOnResponse, TraceLayer},
    LatencyUnit,
};
use tracing::Level;

use crate::{
    configuration::{get_environment, Environment, Settings},
    domain::Url,
    email_client::EmailClient,
};

pub struct Application {
    address: SocketAddr,
    router: Router,
}

impl Application {
    pub fn new(addr: SocketAddr, app_state: AppState) -> Self {
        // Build our application
        let mut router = Router::new()
            .route("/", routing::get(routes::index))
            .route("/health", routing::get(routes::health_check))
            .route("/login", routing::get(routes::login_form))
            .route("/login", routing::post(routes::login))
            .route("/subscribe", routing::post(routes::subscribe))
            .route("/subscribe/confirm", routing::get(routes::confirm))
            .route("/newsletters", routing::post(routes::publish_newsletter))
            .with_state(app_state)
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(|request: &Request<_>| {
                        let trace_id = uuid::Uuid::new_v4().to_string();
                        tracing::info_span!(
                            "request",
                            trace_id = trace_id,
                            method = ?request.method(),
                            uri = %request.uri(),
                            version = ?request.version(),
                        )
                    })
                    .on_response(
                        DefaultOnResponse::new()
                            .level(Level::INFO)
                            .latency_unit(LatencyUnit::Millis),
                    ),
            );

        if let Environment::Local = get_environment() {
            // Fake email server for local env
            router = router.route("/email", routing::post(routes::fake_email))
        };

        Self {
            address: addr,
            router,
        }
    }

    pub fn build(settings: &Settings) -> Self {
        let address = settings
            .application
            .address()
            .expect("Unable to parse socket address.");
        let app_state = default_app_state(settings, None);

        Self::new(address, app_state)
    }

    pub async fn serve(self) -> Result<(), std::io::Error> {
        let listener = tokio::net::TcpListener::bind(self.address).await?;
        tracing::info!("Starting service on {}...", listener.local_addr().unwrap());
        axum::serve(listener, self.router).await
    }

    pub fn router(self) -> Router {
        self.router
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db_pool: Arc<sqlx::PgPool>,
    pub email_client: Arc<EmailClient>,
    pub app_base_url: Url,
}

pub fn default_app_state(settings: &Settings, overwrite_db_pool: Option<sqlx::PgPool>) -> AppState {
    let db_pool = match overwrite_db_pool {
        Some(p) => p,
        None => PgPool::connect_lazy_with(settings.database.with_db()),
    };

    let email_client: EmailClient = settings
        .email_client
        .clone()
        .try_into()
        .expect("Failed to initialize email client.");

    let app_base_url = settings
        .application
        .base_url()
        .expect("Failed to parse application base url.");

    AppState {
        db_pool: Arc::new(db_pool),
        email_client: Arc::new(email_client),
        app_base_url,
    }
}
