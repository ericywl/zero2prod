use std::ops::Deref;

use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use uuid::Uuid;

use crate::{
    session_state::TypedSession,
    utils::{e500, InternalServerError},
};

#[derive(Copy, Clone, Debug)]
pub struct UserId(Uuid);

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Deref for UserId {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub async fn reject_anonymous_users(
    session: TypedSession,
    mut req: Request,
    next: Next,
) -> Result<Response, InternalServerError> {
    match session.get_user_id().await.map_err(e500)? {
        Some(user_id) => {
            req.extensions_mut().insert(UserId(user_id));
            Ok(next.run(req).await.into_response())
        }
        None => Ok(Redirect::to("/login").into_response()),
    }
}
