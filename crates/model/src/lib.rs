use {
  chrono::{DateTime, Utc},
  serde::{Deserialize, Serialize},
  serde_json::Value,
  std::{
    collections::HashSet,
    fmt::{self, Display, Formatter},
    str::FromStr,
  },
  url::Url,
};

mod error;
mod job;
mod job_draft;
mod source;
mod sync_run;

pub use crate::{
  error::Error,
  job::Job,
  job_draft::{EmploymentType, JobDraft, JobLocation, JobSnapshot, Workplace},
  source::Source,
  sync_run::{SyncRun, SyncRunStatus},
};

pub type Result<T = (), E = Error> = std::result::Result<T, E>;
