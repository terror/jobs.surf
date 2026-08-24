use super::*;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EmploymentType {
  Contract,
  FullTime,
  Internship,
  PartTime,
  Temporary,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Workplace {
  Hybrid,
  OnSite,
  Remote,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobDraft {
  pub apply_url: Url,
  pub description_html: Option<String>,
  pub employment_type: Option<EmploymentType>,
  pub external_id: String,
  pub locations: Vec<JobLocation>,
  pub published_at: Option<DateTime<Utc>>,
  pub raw: Value,
  pub title: String,
  pub workplace: Option<Workplace>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobLocation {
  pub name: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobSnapshot {
  pub jobs: Vec<JobDraft>,
}
