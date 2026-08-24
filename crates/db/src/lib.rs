use {
  crate::job_row::JobRow,
  chrono::{DateTime, Utc},
  jobs_surf_model::{
    EmploymentType, Job, JobDraft, JobLocation, JobSnapshot, Source, Workplace,
  },
  sqlx::{PgPool, migrate::MigrateError, postgres::PgPoolOptions, types::Json},
  std::num::{NonZeroU16, TryFromIntError},
};

#[cfg(test)]
use {
  sqlx::{Postgres, migrate::MigrateDatabase, types::JsonValue},
  std::{
    process,
    sync::atomic::{AtomicUsize, Ordering},
    time::{SystemTime, UNIX_EPOCH},
  },
};

mod db;
mod error;
mod job_cursor;
mod job_page;
mod job_record;
mod job_row;
mod source_record;
mod sync_summary;

pub use crate::{
  db::Db, error::Error, job_cursor::JobCursor, job_page::JobPage,
  job_record::JobRecord, source_record::SourceRecord,
  sync_summary::SyncSummary,
};

pub type Result<T = (), E = Error> = std::result::Result<T, E>;

#[cfg(test)]
static TEST_DATABASE_NUMBER: AtomicUsize = AtomicUsize::new(0);
