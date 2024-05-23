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

    pub fn set_query(&mut self, query: Option<&str>) {
        self.0.set_query(query)
    }

    pub fn as_str(&self) -> &str {
        self.as_ref()
    }

    pub fn host_str(&self) -> Option<&str> {
        self.0.host_str()
    }

    pub fn path(&self) -> &str {
        self.0.path()
    }

    pub fn query_params(&self) -> Vec<(String, String)> {
        self.0
            .query_pairs()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }
}

impl ToString for Url {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl AsRef<str> for Url {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}
