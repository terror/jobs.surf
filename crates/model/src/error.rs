use super::*;

#[derive(Debug, Eq, PartialEq, ThisError)]
pub enum Error {
  #[error("invalid sync run status `{0}`")]
  InvalidSyncRunStatus(String),
}
