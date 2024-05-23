use axum::{http::StatusCode, response::IntoResponse};

pub struct HandlerResponse {
    pub status_code: StatusCode,
    pub message: String,
}

impl HandlerResponse {
    pub fn ok() -> Self {
        Self {
            status_code: StatusCode::OK,
            message: "OK".into(),
        }
    }

    pub fn server_error() -> Self {
        Self {
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
            message: "Something wrong with our server".into(),
        }
    }

    pub fn parse_error(message: &str) -> Self {
        Self {
            status_code: StatusCode::UNPROCESSABLE_ENTITY,
            message: message.to_string(),
        }
    }

    pub fn authorization_error(message: &str) -> Self {
        Self {
            status_code: StatusCode::UNAUTHORIZED,
            message: message.to_string(),
        }
    }
}

impl IntoResponse for HandlerResponse {
    fn into_response(self) -> axum::response::Response {
        (self.status_code, self.message).into_response()
    }
}
