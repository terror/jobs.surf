use super::*;

#[derive(Debug, ThisError)]
pub enum Error {
  #[error("failed to connect to postgres")]
  Connect(#[source] sqlx::Error),
  #[error("failed to run database migrations")]
  Migration(#[source] MigrateError),
  #[error(transparent)]
  Query(#[from] sqlx::Error),
}
