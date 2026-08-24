use super::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error(transparent)]
  Ashby(#[from] ashby::Error),
  #[error(transparent)]
  Greenhouse(#[from] greenhouse::Error),
  #[error(transparent)]
  Lever(#[from] lever::Error),
}
