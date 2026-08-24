use {
  chrono::{DateTime, Utc},
  html_escape::decode_html_entities,
  jobs_surf_model::{
    EmploymentType, JobDraft, JobLocation, JobSnapshot, Workplace,
  },
  serde::Deserialize,
  serde_json::Value,
  url::Url,
};

mod adapter;
pub mod ashby;
mod error;
pub mod greenhouse;

pub use crate::{adapter::Adapter, error::Error};

pub type Result<T = (), E = Error> = std::result::Result<T, E>;
