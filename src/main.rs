use zero2prod::configuration::get_configuration;
use zero2prod::startup::run;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let config = get_configuration().expect("Failed to read configuration.");

    let address = format!("127.0.0.1:{}", config.application_port);
    let listener = tokio::net::TcpListener::bind(address).await?;
    run(listener).await
}
