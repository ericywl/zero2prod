use std::fmt::Display;

use thiserror::Error;
use unicode_segmentation::UnicodeSegmentation;

const MAX_SUBSCRIBER_NAME_LENGTH: usize = 256;

#[derive(Debug, Error)]
pub struct ParseNameError(String);

impl AsRef<str> for ParseNameError {
    fn as_ref(&self) -> &str {
        return &self.0;
    }
}

impl Display for ParseNameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

pub struct Name(String);

impl Name {
    /// Returns an instance of `Name` if the input satisfies all
    /// our validation constraints on subscriber names.
    /// It returns `ParseNameError` otherwise.
    pub fn parse(s: String) -> Result<Name, ParseNameError> {
        // `.trim()` returns a view over the input `s` without trailing
        // whitespace-like characters.
        // `.is_empty` checks if the view contains any character.
        let is_empty_or_whitespace = s.trim().is_empty();

        // A grapheme is defined by the Unicode standard as a "user-perceived"
        // character: `å` is a single grapheme, but it is composed of two characters
        // (`a` and `̊`).
        //
        // `graphemes` returns an iterator over the graphemes in the input `s`.
        // `true` specifies that we want to use the extended grapheme definition set,
        // the recommended one.
        let is_too_long = s.graphemes(true).count() > MAX_SUBSCRIBER_NAME_LENGTH;

        // Iterate over all characters in the input `s` to check if any of them
        // matches one of the characters in the forbidden array.
        let forbidden_characters = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
        let contains_forbidden_characters = s.chars().any(|g| forbidden_characters.contains(&g));

        if is_empty_or_whitespace || is_too_long || contains_forbidden_characters {
            Err(ParseNameError(format!(
                "{} is not a valid subscriber name.",
                s
            )))
        } else {
            Ok(Self(s))
        }
    }

    pub fn inner(self) -> String {
        // The caller gets the inner string, but they do not have a SubscriberName anymore!
        // That's because `inner` takes `self` by value, consuming it according to move semantics
        self.0
    }

    pub fn inner_mut(&mut self) -> &mut str {
        &mut self.0
    }
}

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn grapheme_max_length_long_name_is_valid() {
        let name = "ё".repeat(MAX_SUBSCRIBER_NAME_LENGTH);
        assert!(Name::parse(name).is_ok())
    }

    #[test]
    fn name_longer_than_grapheme_max_length_is_rejected() {
        let name = "a".repeat(MAX_SUBSCRIBER_NAME_LENGTH + 1);
        assert!(Name::parse(name).is_err())
    }

    #[test]
    fn whitespace_only_name_is_rejected() {
        let name = " ".to_string();
        assert!(Name::parse(name).is_err())
    }

    #[test]
    fn empty_string_is_rejected() {
        let name = "".to_string();
        assert!(Name::parse(name).is_err());
    }

    #[test]
    fn name_containing_invalid_characters_is_rejected() {
        for name in &['/', '(', ')', '"', '<', '>', '\\', '{', '}'] {
            let name = name.to_string();
            assert!(Name::parse(name).is_err());
        }
    }

    #[test]
    fn valid_name_is_parsed_successfully() {
        let name = "Le Pomodoro Primo Passo".to_string();
        assert!(Name::parse(name).is_ok())
    }
}
