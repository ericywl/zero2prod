use std::sync::Arc;

use axum_test::TestServer;
use once_cell::sync::Lazy;
use sqlx::PgPool;

use zero2prod::{
    startup::AppState,
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

async fn app_state(pool: PgPool) -> Arc<AppState> {
    Arc::new(AppState { db_pool: pool })
}

fn test_server(state: Arc<AppState>) -> TestServer {
    let app = zero2prod::startup::app(state);
    TestServer::new(app).expect("Failed to spawn test server")
}

pub struct TestSetup {
    pub server: TestServer,
    pub app_state: Arc<AppState>,
}

pub async fn test_setup(pool: PgPool) -> TestSetup {
    Lazy::force(&TRACING);

    let app_state = app_state(pool).await;
    let server = test_server(app_state.clone());

    TestSetup { server, app_state }
}
