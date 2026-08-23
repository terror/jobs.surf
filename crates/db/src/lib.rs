use {
  jobs_surf_model::{Job, Source},
  sqlx::{PgPool, migrate::MigrateError, postgres::PgPoolOptions, types::Json},
  thiserror::Error as ThisError,
};

#[cfg(test)]
use {
  sqlx::{Postgres, migrate::MigrateDatabase},
  std::{
    process,
    sync::atomic::{AtomicUsize, Ordering},
    time::{SystemTime, UNIX_EPOCH},
  },
};

mod db;
mod error;

pub use crate::{db::Db, error::Error};

pub type Result<T = (), E = Error> = std::result::Result<T, E>;

#[cfg(test)]
static TEST_DATABASE_NUMBER: AtomicUsize = AtomicUsize::new(0);
