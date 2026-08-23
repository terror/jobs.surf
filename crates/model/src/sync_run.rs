use super::*;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncRunStatus {
  Running,
  Succeeded,
  Failed,
}

impl SyncRunStatus {
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Running => "running",
      Self::Succeeded => "succeeded",
      Self::Failed => "failed",
    }
  }
}

impl Display for SyncRunStatus {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    f.write_str(self.as_str())
  }
}

impl FromStr for SyncRunStatus {
  type Err = Error;

  fn from_str(value: &str) -> Result<Self> {
    match value {
      "running" => Ok(Self::Running),
      "succeeded" => Ok(Self::Succeeded),
      "failed" => Ok(Self::Failed),
      _ => Err(Error::InvalidSyncRunStatus(value.into())),
    }
  }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncRun {
  pub source_id: String,
  pub status: SyncRunStatus,
  pub jobs_seen: i32,
  pub jobs_upserted: i32,
  pub jobs_closed: i32,
  pub error: Option<String>,
  pub started_at: DateTime<Utc>,
  pub finished_at: Option<DateTime<Utc>>,
}
