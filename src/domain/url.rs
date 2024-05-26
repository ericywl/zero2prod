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

#[derive(Debug, Clone)]
pub struct Url(reqwest::Url);

impl Url {
    /// Returns an instance of `Url` if the input satisfies all our validation constraints on URLs.
    /// It returns `ParseUrlError` otherwise.
    pub fn parse(s: &str) -> Result<Self, ParseUrlError> {
        match reqwest::Url::parse(s) {
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

impl Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for Url {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn invalid_url_is_rejected() {
        assert!(Url::parse("this-is-not-a-url").is_err())
    }

    #[test]
    fn valid_url_is_parsed_successfully() {
        assert!(Url::parse("https://some-url.com/hello").is_ok())
    }

    #[test]
    fn url_returns_proper_host_str() {
        let url = Url::parse("http://my-domain.com/do-something").expect("Failed to parse url.");
        assert_eq!(url.host_str().unwrap(), "my-domain.com");
    }

    #[test]
    fn url_returns_proper_path() {
        let url = Url::parse("http://my-domain.com/do-something").expect("Failed to parse url.");
        assert_eq!(url.path(), "/do-something");
    }

    #[test]
    fn url_returns_proper_query_params() {
        let url = Url::parse("http://some-url.co.jp/anime_character?name=sasuke&is_cool=true")
            .expect("Failed to parse url.");
        assert_eq!(
            url.query_params(),
            vec![
                ("name".into(), "sasuke".into()),
                ("is_cool".into(), "true".into())
            ]
        );
    }
}
