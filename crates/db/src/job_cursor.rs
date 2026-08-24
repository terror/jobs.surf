use chrono::{DateTime, Utc};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JobCursor {
  pub first_seen_at: DateTime<Utc>,
  pub id: i64,
}
