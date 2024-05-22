mod common;

use axum::http::StatusCode;
use sqlx::PgPool;

#[cfg(test)]
#[sqlx::test]
async fn subscribe_returns_200_for_valid_form_data(pool: PgPool) {
    // Arrange
    let setup = common::test_setup(pool).await;
    let mut connection = setup
        .app_state
        .db_pool
        .acquire()
        .await
        .expect("Failed to get connection from pool.");

    // Act
    let name = "Bob Banjo";
    let email = "bob_banjo@gmail.com";
    let body = &[("name", name), ("email", email)];
    let response = setup.server.post("/subscribe").form(body).await;

    // Assert
    assert_eq!(response.status_code(), StatusCode::OK);

    let saved = sqlx::query!("SELECT name, email FROM subscriptions",)
        .fetch_one(&mut *connection)
        .await
        .expect("Failed to fetch saved subscription");
    assert_eq!(saved.name, name);
    assert_eq!(saved.email, email);
}

#[cfg(test)]
#[sqlx::test]
async fn subscribe_returns_400_when_data_is_missing(pool: PgPool) {
    // Arrange
    let setup = common::test_setup(pool).await;
    let test_cases: Vec<(&str, &[(&str, &str)])> = vec![
        ("missing the email", &[("name", "Bob Banjo")]),
        ("missing the name", &[("email", "bob_banjo@gmail.com")]),
        ("missing both name and email", &[]),
        (
            "invalid email",
            &[("name", "Miquella"), ("email", "definitely-not-email")],
        ),
    ];

    for (error_message, invalid_body) in test_cases {
        // Act
        let response = setup.server.post("/subscribe").form(invalid_body).await;

        // Assert
        assert_eq!(
            response.status_code(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "API did not fail with 400 Bad Request when payload was {}",
            error_message
        );
    }
}
