use super::*;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
  pub apply_url: String,
  pub description_html: Option<String>,
  pub employment_type: Option<String>,
  pub external_id: String,
  pub locations: Vec<JobLocation>,
  pub published_at: Option<DateTime<Utc>>,
  pub raw: Value,
  pub source_id: String,
  pub title: String,
  pub workplace: Option<String>,
}
