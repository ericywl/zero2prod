use validator::ValidateEmail;

pub struct SubscriberEmail(String);

impl SubscriberEmail {
    /// Returns an instance of `SubscriberEmail` if the input satisfies all
    /// our validation constraints on subscriber emails.
    /// It returns `SubscriberEmailParseError` otherwise.
    pub fn parse(s: String) -> Result<SubscriberEmail, String> {
        let email = Self(s.clone());
        if !email.validate_email() {
            Err(format!("{} is not a valid subscriber email.", s))
        } else {
            Ok(email)
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl ValidateEmail for SubscriberEmail {
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
        let email = "".to_string();
        assert!(SubscriberEmail::parse(email).is_err());
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "mydomain.com".to_string();
        assert!(SubscriberEmail::parse(email).is_err());
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@domain.com".to_string();
        assert!(SubscriberEmail::parse(email).is_err());
    }

    #[quickcheck_macros::quickcheck]
    fn valid_email_is_parsed_successfully(valid_email: ValidEmailFixture) {
        assert!(SubscriberEmail::parse(valid_email.0).is_ok());
    }
}
