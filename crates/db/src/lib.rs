use {
  jobs_surf_model::{
    EmploymentType, Job, JobDraft, JobSnapshot, Source, Workplace,
  },
  sqlx::{PgPool, migrate::MigrateError, postgres::PgPoolOptions, types::Json},
  std::num::TryFromIntError,
  thiserror::Error as ThisError,
};

#[cfg(test)]
use {
  jobs_surf_model::JobLocation,
  sqlx::{Postgres, migrate::MigrateDatabase, types::JsonValue},
  std::{
    process,
    sync::atomic::{AtomicUsize, Ordering},
    time::{SystemTime, UNIX_EPOCH},
  },
};

mod db;
mod error;

pub use crate::{
  db::{Db, SyncSummary},
  error::Error,
};

pub type Result<T = (), E = Error> = std::result::Result<T, E>;

#[cfg(test)]
static TEST_DATABASE_NUMBER: AtomicUsize = AtomicUsize::new(0);
