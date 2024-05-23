use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use thiserror::Error;

#[derive(strum_macros::Display)]
#[strum(serialize_all = "snake_case")]
pub enum SubscriptionStatus {
    PendingConfirmation,
    Confirmed,
}

#[derive(Debug, Error)]
pub enum ParseSubscriptionTokenError {
    #[error("invalid token length")]
    InvalidLength,

    #[error("token not alphanumeric")]
    NotAlphanumeric,
}

pub struct SubscriptionToken(String);

impl SubscriptionToken {
    const TOKEN_LENGTH: usize = 25;

    /// Returns an instance of `SubscriptionToken` if the input satisfies all our validation constraints on subscription token.
    /// It returns `ParseSubscriptionTokenError` otherwise.
    pub fn parse(s: &str) -> Result<Self, ParseSubscriptionTokenError> {
        if !s.chars().all(char::is_alphanumeric) {
            return Err(ParseSubscriptionTokenError::NotAlphanumeric);
        }

        if s.chars().count() != Self::TOKEN_LENGTH {
            return Err(ParseSubscriptionTokenError::InvalidLength);
        }

        Ok(Self(s.to_string()))
    }

    /// Generate a random 25-characters-long case-sensitive subscription token.
    pub fn generate() -> Self {
        let mut rng = thread_rng();
        Self(
            std::iter::repeat_with(|| rng.sample(Alphanumeric))
                .map(char::from)
                .take(25)
                .collect(),
        )
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn subscription_token_that_is_not_alphanumeric_is_rejected() {
        assert!(SubscriptionToken::parse("this=-not!@$alphanumeric").is_err());
    }

    #[test]
    fn subscription_token_that_is_invalid_length_is_rejected() {
        assert!(SubscriptionToken::parse("short").is_err());
        assert!(SubscriptionToken::parse("thisiswaytooooolooooooooooooooooooong11111").is_err());
    }

    #[test]
    fn valid_subscription_token_is_parsed_successfully() {
        assert!(SubscriptionToken::parse("vC8nGu4tq3DwcXu5rhLXa0Y7S").is_ok());
    }
}
