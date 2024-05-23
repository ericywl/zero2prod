#[derive(strum_macros::Display)]
#[strum(serialize_all = "snake_case")]
pub enum SubscriptionStatus {
    PendingConfirmation,
    Confirmed,
}
