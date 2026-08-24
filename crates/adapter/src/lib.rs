use {
  html_escape::decode_html_entities,
  jobs_surf_model::{JobDraft, JobLocation, JobSnapshot},
  serde::Deserialize,
  serde_json::Value,
  url::Url,
};

mod adapter;
mod error;
pub mod greenhouse;

pub use crate::{adapter::Adapter, error::Error};

pub type Result<T = (), E = Error> = std::result::Result<T, E>;
