use std::fmt::Display;

use thiserror::Error;
use validator::ValidateEmail;

#[derive(Debug, Error)]
pub struct ParseEmailError(String);

impl AsRef<str> for ParseEmailError {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Display for ParseEmailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

#[derive(Debug)]
pub struct Email(String);

impl Email {
    /// Returns an instance of `Email` if the input satisfies all our validation constraints on emails.
    /// It returns `ParseEmailError` otherwise.
    pub fn parse(s: &str) -> Result<Email, ParseEmailError> {
        let email = Self(s.to_string());
        if !email.validate_email() {
            Err(ParseEmailError(format!(
                "{} is not a valid subscriber email.",
                s
            )))
        } else {
            Ok(email)
        }
    }

    pub fn as_str(&self) -> &str {
        self.as_ref()
    }
}

impl std::fmt::Display for Email {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for Email {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl ValidateEmail for Email {
    fn as_email_string(&self) -> Option<std::borrow::Cow<str>> {
        Some(std::borrow::Cow::Borrowed(self.as_ref()))
    }
}

#[cfg(test)]
mod test {
    // We are importing the `SafeEmail` faker!
    // We also need the `Fake` trait to get access to the
    // `.fake` method on `SafeEmail`
    use fake::faker::internet::en::SafeEmail;
    use fake::Fake;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    use super::*;

    #[derive(Debug, Clone)]
    struct ValidEmailFixture(pub String);

    impl quickcheck::Arbitrary for ValidEmailFixture {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            let mut rng = StdRng::seed_from_u64(u64::arbitrary(g));
            let email = SafeEmail().fake_with_rng(&mut rng);

            Self(email)
        }
    }

    #[test]
    fn empty_email_is_rejected() {
        let email = "";
        assert!(Email::parse(email).is_err());
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "mydomain.com";
        assert!(Email::parse(email).is_err());
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@domain.com";
        assert!(Email::parse(email).is_err());
    }

    #[quickcheck_macros::quickcheck]
    fn valid_email_is_parsed_successfully(valid_email: ValidEmailFixture) {
        assert!(Email::parse(&valid_email.0).is_ok());
    }
}
