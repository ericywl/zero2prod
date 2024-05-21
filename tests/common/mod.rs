use std::sync::Arc;

use axum_test::TestServer;
use sqlx::PgPool;
use zero2prod::startup::AppState;

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
    let app_state = app_state(pool).await;
    let server = test_server(app_state.clone());

    TestSetup { server, app_state }
}
