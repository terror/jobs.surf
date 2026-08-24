use super::*;

#[derive(Debug, ThisError)]
pub enum Error {
  #[error("failed to connect to postgres")]
  Connect(#[source] sqlx::Error),
  #[error("sync count exceeds the supported range")]
  CountOverflow(#[source] TryFromIntError),
  #[error("failed to run database migrations")]
  Migration(#[source] MigrateError),
  #[error(transparent)]
  Query(#[from] sqlx::Error),
  #[error("sync run `{0}` is not running for the requested source")]
  SyncRunNotRunning(i64),
  #[error("sync run `{0}` was superseded by a newer successful run")]
  SyncRunSuperseded(i64),
}
