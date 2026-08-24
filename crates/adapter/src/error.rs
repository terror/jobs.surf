use super::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error(transparent)]
  Greenhouse(#[from] greenhouse::Error),
}
