use super::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error(transparent)]
  Ashby(#[from] ashby::Error),
  #[error(transparent)]
  Breezy(#[from] breezy::Error),
  #[error(transparent)]
  Comeet(#[from] comeet::Error),
  #[error(transparent)]
  Greenhouse(#[from] greenhouse::Error),
  #[error(transparent)]
  Lever(#[from] lever::Error),
  #[error(transparent)]
  Workable(#[from] workable::Error),
}
