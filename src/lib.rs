use {
  jobs_surf_adapter::{
    Adapter, ashby::Ashby, breezy::Breezy, comeet::Comeet,
    greenhouse::Greenhouse, lever::Lever, personio::Personio,
    recruitee::Recruitee, teamtailor::Teamtailor, workable::Workable,
  },
  serde::Deserialize,
  serde_json::Value,
};

pub mod config;
