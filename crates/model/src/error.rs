#[derive(Debug, Eq, PartialEq, thiserror::Error)]
pub enum Error {
  #[error("provider returned duplicate external ID `{0}`")]
  DuplicateJobExternalId(String),
  #[error("provider returned a job with an empty external ID")]
  EmptyJobExternalId,
  #[error("provider returned job `{0}` with an empty title")]
  EmptyJobTitle(String),
  #[error("invalid sync run status `{0}`")]
  InvalidSyncRunStatus(String),
}
