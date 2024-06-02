use axum::response::{IntoResponse, Redirect};
use axum_flash::Flash;

use crate::session_state::TypedSession;

pub async fn admin_logout(flash: Flash, session: TypedSession) -> impl IntoResponse {
    session.logout().await;
    (
        flash.success("You have successfully logged out"),
        Redirect::to("/login"),
    )
}
