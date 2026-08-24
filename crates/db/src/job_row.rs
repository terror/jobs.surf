use super::*;

#[derive(sqlx::FromRow)]
pub(crate) struct JobRow {
  pub(crate) apply_url: String,
  pub(crate) description_html: Option<String>,
  pub(crate) employment_type: Option<String>,
  pub(crate) first_seen_at: DateTime<Utc>,
  pub(crate) id: i64,
  pub(crate) locations: Json<Vec<JobLocation>>,
  pub(crate) published_at: Option<DateTime<Utc>>,
  pub(crate) source_id: String,
  pub(crate) title: String,
  pub(crate) workplace: Option<String>,
}
