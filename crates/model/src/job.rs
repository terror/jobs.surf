use super::*;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
  pub source_id: String,
  pub external_id: String,
  pub title: String,
  pub description_html: Option<String>,
  pub apply_url: String,
  pub locations: Vec<String>,
  pub workplace: Option<String>,
  pub employment_type: Option<String>,
  pub published_at: Option<DateTime<Utc>>,
  pub raw: Value,
}
