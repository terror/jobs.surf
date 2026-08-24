use {super::*, std::collections::HashSet};

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

impl EmploymentType {
  #[must_use]
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Contract => "contract",
      Self::FullTime => "full_time",
      Self::Internship => "internship",
      Self::PartTime => "part_time",
      Self::Temporary => "temporary",
    }
  }
}

impl JobSnapshot {
  /// Validates invariants required to persist this complete snapshot safely.
  ///
  /// # Errors
  ///
  /// Returns an error if a job has an empty external ID, duplicate external
  /// ID, or empty title.
  pub fn validate(&self) -> Result {
    let mut external_ids = HashSet::with_capacity(self.jobs.len());

    for job in &self.jobs {
      if job.external_id.trim().is_empty() {
        return Err(Error::EmptyJobExternalId);
      }

      if !external_ids.insert(&job.external_id) {
        return Err(Error::DuplicateJobExternalId(job.external_id.clone()));
      }

      if job.title.trim().is_empty() {
        return Err(Error::EmptyJobTitle(job.external_id.clone()));
      }
    }

    Ok(())
  }
}

impl Workplace {
  #[must_use]
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Hybrid => "hybrid",
      Self::OnSite => "on_site",
      Self::Remote => "remote",
    }
  }
}
