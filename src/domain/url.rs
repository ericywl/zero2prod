use std::fmt::Display;

use thiserror::Error;

#[derive(Debug, Error)]
pub struct ParseUrlError(String);

impl AsRef<str> for ParseUrlError {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Display for ParseUrlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

pub struct Url(reqwest::Url);

impl Url {
    /// Returns an instance of `Url` if the input satisfies all
    /// our validation constraints on subscriber emails.
    /// It returns `ParseUrlError` otherwise.
    pub fn parse(s: String) -> Result<Self, ParseUrlError> {
        match reqwest::Url::parse(&s) {
            Ok(url) => Ok(Self(url)),
            Err(e) => Err(ParseUrlError(e.to_string())),
        }
    }

    pub fn join(&self, s: &str) -> Result<Url, ParseUrlError> {
        match self.0.join(s) {
            Ok(url) => Ok(Self(url)),
            Err(e) => Err(ParseUrlError(e.to_string())),
        }
    }

    pub fn inner(&self) -> reqwest::Url {
        self.0.clone()
    }
}
