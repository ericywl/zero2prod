use std::{net::SocketAddr, sync::Arc};

use super::routes;
use axum::{http::Request, middleware, routing, Router};
use secrecy::ExposeSecret;
use sqlx::PgPool;
use tower_http::{
    trace::{DefaultOnResponse, TraceLayer},
    LatencyUnit,
};
use tower_sessions::SessionManagerLayer;
use tower_sessions_redis_store::{
    fred::{clients::RedisPool, interfaces::ClientLike, types::RedisConfig},
    RedisStore,
};
use tracing::Level;

use crate::{
    authentication::reject_anonymous_users,
    configuration::{get_environment, Environment, Settings},
    domain::Url,
    email_client::EmailClient,
};

pub struct Application {
    address: SocketAddr,
    router: Router,
}

impl Application {
    pub fn new(
        addr: SocketAddr,
        app_state: AppState,
        session_layer: SessionManagerLayer<RedisStore<RedisPool>>,
    ) -> Self {
        // Normal user routes
        let mut app_router = Router::new()
            .route("/", routing::get(routes::index))
            .route("/", routing::post(routes::subscribe_with_flash))
            .route("/health", routing::get(routes::health_check))
            .route("/login", routing::get(routes::login_form))
            .route("/login", routing::post(routes::login_with_flash))
            .route("/subscribe", routing::post(routes::subscribe))
            .route("/subscribe/confirm", routing::get(routes::confirm));
        if let Environment::Local = get_environment() {
            // Fake email server for local env
            app_router = app_router.route("/email", routing::post(routes::fake_email))
        };

        // Admin routes
        let admin_router = Router::new()
            .route("/admin/dashboard", routing::get(routes::admin_dashboard))
            .route(
                "/admin/password",
                routing::get(routes::change_password_form),
            )
            .route(
                "/admin/password",
                routing::post(routes::change_password_with_flash),
            )
            .route("/admin/logout", routing::post(routes::admin_logout))
            .route(
                "/admin/newsletters",
                routing::post(routes::publish_newsletter),
            )
            .layer(middleware::from_fn(reject_anonymous_users));

        // Build our application
        let router = app_router
            .merge(admin_router)
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
            )
            .layer(session_layer);

        Self {
            address: addr,
            router,
        }
    }

    pub async fn build(settings: &Settings) -> Self {
        let address = settings
            .application
            .address()
            .expect("Unable to parse socket address.");
        let (app_state, session_layer) = default_app_state_and_session(settings, None).await;

        Self::new(address, app_state, session_layer)
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
    pub flash_config: axum_flash::Config,
}

impl axum::extract::FromRef<AppState> for axum_flash::Config {
    fn from_ref(state: &AppState) -> axum_flash::Config {
        state.flash_config.clone()
    }
}

pub async fn default_app_state_and_session(
    settings: &Settings,
    overwrite_db_pool: Option<sqlx::PgPool>,
) -> (AppState, SessionManagerLayer<RedisStore<RedisPool>>) {
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

    // Initialize Redis session
    let redis_config = RedisConfig::from_url(settings.redis_uri.expose_secret())
        .expect("Unable to parse redis URI.");
    let redis_pool = RedisPool::new(redis_config, None, None, None, 6)
        .expect("Unable to initialize redis pool.");

    redis_pool.connect();
    redis_pool
        .wait_for_connect()
        .await
        .expect("Unable to connect to pool.");

    let session_store = RedisStore::new(redis_pool);
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_expiry(tower_sessions::Expiry::OnInactivity(
            time::Duration::minutes(10),
        ));

    (
        AppState {
            db_pool: Arc::new(db_pool),
            email_client: Arc::new(email_client),
            app_base_url,
            flash_config: axum_flash::Config::new(axum_flash::Key::generate()),
        },
        session_layer,
    )
}
