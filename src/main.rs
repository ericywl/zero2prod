use zero2prod::configuration::get_configuration;
use zero2prod::startup::Application;
use zero2prod::telemetry;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let subscriber = telemetry::get_subscriber(
        "zero2prod".into(),
        "info,axum::rejection=trace".into(),
        std::io::stdout,
    );
    telemetry::init_subscriber(subscriber);

    let settings = get_configuration().expect("Failed to read configuration.");
    let app = Application::build(&settings);
    app.serve().await
}
