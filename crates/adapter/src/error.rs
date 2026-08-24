use super::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error(transparent)]
  Ashby(#[from] ashby::Error),
  #[error(transparent)]
  Breezy(#[from] breezy::Error),
  #[error(transparent)]
  Comeet(#[from] comeet::Error),
  #[error("failed to fetch jobs from {adapter} source `{identifier}`")]
  Fetch {
    adapter: &'static str,
    identifier: String,
    #[source]
    source: reqwest::Error,
  },
  #[error(transparent)]
  Greenhouse(#[from] greenhouse::Error),
  #[error(transparent)]
  Lever(#[from] lever::Error),
  #[error("failed to parse URL `{url}` for {adapter} source `{identifier}`")]
  ParseUrl {
    adapter: &'static str,
    identifier: String,
    #[source]
    source: url::ParseError,
    url: String,
  },
  #[error(transparent)]
  Personio(#[from] personio::Error),
  #[error(transparent)]
  Recruitee(#[from] recruitee::Error),
  #[error(transparent)]
  Teamtailor(#[from] teamtailor::Error),
  #[error(transparent)]
  Workable(#[from] workable::Error),
}
