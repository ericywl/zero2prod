use axum::{
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use axum_flash::Flash;

use crate::session_state::TypedSession;

pub async fn admin_logout(flash: Flash, session: TypedSession) -> Response {
    let user_id = match session.get_user_id().await {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Something went wrong with logout".to_string(),
            )
                .into_response()
        }
    };

    if user_id.is_none() {
        Redirect::to("/login").into_response()
    } else {
        session.logout().await;
        (
            flash.success("You have successfully logged out"),
            Redirect::to("/login"),
        )
            .into_response()
    }
}
