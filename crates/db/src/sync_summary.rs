#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SyncSummary {
  pub jobs_closed: i32,
  pub jobs_seen: i32,
  pub jobs_upserted: i32,
}
