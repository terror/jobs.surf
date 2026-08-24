use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JobRecord {
  pub apply_url: String,
  pub description_html: Option<String>,
  pub employment_type: Option<String>,
  pub first_seen_at: DateTime<Utc>,
  pub id: i64,
  pub locations: Vec<JobLocation>,
  pub published_at: Option<DateTime<Utc>>,
  pub source_id: String,
  pub title: String,
  pub workplace: Option<String>,
}

impl From<JobRow> for JobRecord {
  fn from(row: JobRow) -> Self {
    Self {
      apply_url: row.apply_url,
      description_html: row.description_html,
      employment_type: row.employment_type,
      first_seen_at: row.first_seen_at,
      id: row.id,
      locations: row.locations.0,
      published_at: row.published_at,
      source_id: row.source_id,
      title: row.title,
      workplace: row.workplace,
    }
  }
}
