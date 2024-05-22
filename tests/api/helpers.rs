use std::sync::Arc;

use axum_test::TestServer;
use once_cell::sync::Lazy;
use sqlx::PgPool;

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

pub struct TestSetup {
    pub server: TestServer,
    pub app_state: Arc<AppState>,
}

pub async fn test_setup(pool: PgPool) -> TestSetup {
    Lazy::force(&TRACING);

    let config = get_configuration().expect("Failed to read configuration.");

    let app_state = Arc::new(default_app_state(&config, Some(pool)));
    let address = config
        .application
        .address()
        .expect("Failed to parse address.");

    let app = zero2prod::startup::Application::new(address, app_state.clone());
    let server = TestServer::new(app.router()).expect("Failed to spawn test server");

    TestSetup { server, app_state }
}
