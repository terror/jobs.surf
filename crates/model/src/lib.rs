use {
  chrono::{DateTime, Utc},
  serde::{Deserialize, Serialize},
  serde_json::Value,
  std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
  },
  thiserror::Error as ThisError,
};

mod error;
mod job;
mod source;
mod sync_run;

pub use crate::{
  error::Error,
  job::Job,
  source::Source,
  sync_run::{SyncRun, SyncRunStatus},
};

pub type Result<T = (), E = Error> = std::result::Result<T, E>;
