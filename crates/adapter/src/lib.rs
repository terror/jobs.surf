use {
  chrono::{DateTime, NaiveDate, NaiveDateTime, Utc},
  html_escape::{decode_html_entities, encode_text},
  jobs_surf_model::{
    EmploymentType, JobDraft, JobLocation, JobSnapshot, Workplace,
  },
  serde::Deserialize,
  serde_json::Value,
  url::Url,
};

mod adapter;
pub mod ashby;
pub mod breezy;
pub mod comeet;
mod error;
pub mod greenhouse;
pub mod lever;
pub mod recruitee;
pub mod workable;

pub use crate::{adapter::Adapter, error::Error};

pub type Result<T = (), E = Error> = std::result::Result<T, E>;
