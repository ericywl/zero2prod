use std::sync::Arc;

use axum_test::TestServer;
use sqlx::PgPool;
use zero2prod::{configuration::get_configuration, startup::AppState};

async fn test_app_state() -> Arc<AppState> {
    let config = get_configuration().expect("Failed to read configuration.");
    let pool = PgPool::connect(&config.database.connection_string())
        .await
        .expect("Failed to connect to Postgres.");

    Arc::new(AppState { db_pool: pool })
}

fn test_server(state: Arc<AppState>) -> TestServer {
    let app = zero2prod::startup::app(state);
    TestServer::new(app).expect("Failed to spawn test server")
}

pub async fn test_setup() -> (TestServer, Arc<AppState>) {
    let state = test_app_state().await;
    let server = test_server(state.clone());

    (server, state)
}
