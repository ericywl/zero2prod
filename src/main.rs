use zero2prod::run;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    run(listener).await
}
