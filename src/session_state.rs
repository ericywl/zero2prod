use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use tower_sessions::{session, Session};
use uuid::Uuid;

pub struct TypedSession(Session);

impl TypedSession {
    const USER_ID_KEY: &'static str = "user_id";

    pub async fn renew(&self) -> Result<(), session::Error> {
        self.0.cycle_id().await
    }

    pub async fn insert_user_id(&self, user_id: Uuid) -> Result<(), session::Error> {
        self.0.insert(Self::USER_ID_KEY, user_id).await
    }

    pub async fn get_user_id(&self) -> Result<Option<Uuid>, session::Error> {
        self.0.get(Self::USER_ID_KEY).await
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for TypedSession
where
    S: Sync + Send,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Ok(TypedSession(
            Session::from_request_parts(parts, state).await?,
        ))
    }
}
