use axum_test::TestServer;

pub fn test_server() -> TestServer {
    let app = zero2prod::app();
    TestServer::new(app).expect("Failed to spawn test server")
}
