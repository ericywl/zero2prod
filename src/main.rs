use std::sync::Arc;

use secrecy::ExposeSecret;
use sqlx::PgPool;
use zero2prod::configuration::{get_configuration, Settings};
use zero2prod::startup::{run, AppState};
use zero2prod::telemetry;

pub async fn app_state(settings: &Settings) -> Arc<AppState> {
    let pool = PgPool::connect(&settings.database.connection_string().expose_secret())
        .await
        .expect("Failed to connect to Postgres.");

    Arc::new(AppState { db_pool: pool })
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let subscriber = telemetry::get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    telemetry::init_subscriber(subscriber);

    let config = get_configuration().expect("Failed to read configuration.");
    let state = app_state(&config).await;
    let address = format!("127.0.0.1:{}", config.application_port);
    let listener = tokio::net::TcpListener::bind(address).await?;

    run(listener, state).await
}
