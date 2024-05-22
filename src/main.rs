use std::sync::Arc;

use sqlx::PgPool;
use zero2prod::configuration::{get_configuration, Settings};
use zero2prod::email_client::EmailClient;
use zero2prod::startup::{run, AppState};
use zero2prod::telemetry;

pub async fn app_state(settings: &Settings) -> Arc<AppState> {
    let db_pool = PgPool::connect_lazy_with(settings.database.with_db());
    let email_client: EmailClient = settings
        .email_client
        .clone()
        .try_into()
        .expect("Failed to initialize email client.");

    Arc::new(AppState {
        db_pool,
        email_client,
    })
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let subscriber = telemetry::get_subscriber(
        "zero2prod".into(),
        "info,axum::rejection=trace".into(),
        std::io::stdout,
    );
    telemetry::init_subscriber(subscriber);

    let config = get_configuration().expect("Failed to read configuration.");
    let state = app_state(&config).await;
    let address = format!("{}:{}", config.application.host, config.application.port);
    let listener = tokio::net::TcpListener::bind(address).await?;

    run(listener, state).await
}
