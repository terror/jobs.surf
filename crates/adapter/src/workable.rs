use super::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("failed to decode jobs for Workable account `{account}`")]
  Decode {
    account: String,
    #[source]
    source: serde_json::Error,
  },
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ProviderWorkplace {
  Hybrid,
  OnSite,
  Remote,
}

#[derive(Deserialize)]
struct ProviderLocation {
  city: Option<String>,
  country: Option<String>,
  hidden: Option<bool>,
  region: Option<String>,
}

#[derive(Deserialize)]
struct ProviderJob {
  application_url: Url,
  city: Option<String>,
  country: Option<String>,
  description: Option<String>,
  employment_type: Option<String>,
  #[serde(default)]
  locations: Vec<ProviderLocation>,
  published_on: Option<NaiveDate>,
  shortcode: String,
  state: Option<String>,
  telecommuting: Option<bool>,
  title: String,
  workplace_type: Option<ProviderWorkplace>,
}

#[derive(Deserialize)]
struct Response {
  jobs: Vec<Value>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Workable {
  account: String,
}

impl Workable {
  #[must_use]
  pub fn account(&self) -> &str {
    &self.account
  }

  #[must_use]
  pub fn new(account: impl Into<String>) -> Self {
    Self {
      account: account.into(),
    }
  }
}

#[async_trait::async_trait]
impl Adapter for Workable {
  async fn fetch(&self) -> Result<JobSnapshot> {
    let client = reqwest::Client::new();

    let mut url = http::parse_url(
      "Workable",
      &self.account,
      format!("https://www.workable.com/api/accounts/{}", self.account),
    )?;

    url.query_pairs_mut().append_pair("details", "true");

    let response = http::get(&client, "Workable", &self.account, url).await?;

    self.normalize(&response)
  }

  fn normalize(&self, response: &[u8]) -> Result<JobSnapshot> {
    let response: Response =
      serde_json::from_slice(response).map_err(|source| Error::Decode {
        account: self.account.clone(),
        source,
      })?;

    let normalized = response
      .jobs
      .into_iter()
      .map(|raw| {
        let job: ProviderJob =
          serde_json::from_value(raw.clone()).map_err(|source| {
            Error::Decode {
              account: self.account.clone(),
              source,
            }
          })?;

        let employment_type =
          job.employment_type.as_deref().and_then(|employment_type| {
            match employment_type.to_ascii_lowercase().as_str() {
              "contract" | "contractor" => Some(EmploymentType::Contract),
              "full time" | "full-time" | "fulltime" => {
                Some(EmploymentType::FullTime)
              }
              "intern" | "internship" => Some(EmploymentType::Internship),
              "part time" | "part-time" | "parttime" => {
                Some(EmploymentType::PartTime)
              }
              "temp" | "temporary" => Some(EmploymentType::Temporary),
              _ => None,
            }
          });

        let locations = if job.locations.is_empty() {
          vec![
            [job.city, job.state, job.country]
              .into_iter()
              .flatten()
              .filter(|part| !part.trim().is_empty())
              .collect::<Vec<_>>()
              .join(", "),
          ]
        } else {
          job
            .locations
            .into_iter()
            .filter(|location| location.hidden != Some(true))
            .filter_map(|location| {
              let name = [location.city, location.region, location.country]
                .into_iter()
                .flatten()
                .filter(|part| !part.trim().is_empty())
                .collect::<Vec<_>>()
                .join(", ");

              (!name.is_empty()).then_some(name)
            })
            .collect()
        };

        let locations = locations
          .into_iter()
          .filter(|name| !name.is_empty())
          .map(|name| JobLocation { name })
          .collect();

        let workplace = match (job.workplace_type, job.telecommuting) {
          (Some(ProviderWorkplace::Hybrid), _) => Some(Workplace::Hybrid),
          (Some(ProviderWorkplace::OnSite), _) => Some(Workplace::OnSite),
          (Some(ProviderWorkplace::Remote), _) | (None, Some(true)) => {
            Some(Workplace::Remote)
          }
          (None, Some(false) | None) => None,
        };

        Ok(JobDraft {
          apply_url: job.application_url,
          description_html: job
            .description
            .filter(|description| !description.is_empty()),
          employment_type,
          external_id: job.shortcode,
          locations,
          published_at: job
            .published_on
            .and_then(|date| date.and_hms_opt(0, 0, 0))
            .map(|date| date.and_utc()),
          raw,
          title: job.title,
          workplace,
        })
      })
      .collect::<Result<Vec<_>, Error>>()?;

    let mut jobs: Vec<JobDraft> = Vec::with_capacity(normalized.len());
    let mut indexes = HashMap::<String, usize>::new();

    for mut job in normalized {
      if let Some(index) = indexes.get(&job.external_id).copied() {
        let locations = &jobs[index].locations;
        job.locations.retain(|item| !locations.contains(item));
        jobs[index].locations.extend(job.locations);
      } else {
        indexes.insert(job.external_id.clone(), jobs.len());
        jobs.push(job);
      }
    }

    Ok(JobSnapshot { jobs })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const FIXTURE: &[u8] = include_bytes!("../tests/fixtures/workable/jobs.json");

  #[test]
  fn normalizes_jobs() {
    let adapter = Workable::new("acme");

    let response: Value = serde_json::from_slice(FIXTURE).unwrap();

    assert_eq!(
      adapter.normalize(FIXTURE).unwrap(),
      JobSnapshot {
        jobs: vec![
          JobDraft {
            apply_url: Url::parse(
              "https://apply.workable.com/j/F4C096B22E/apply",
            )
            .unwrap(),
            description_html: Some(
              "<p>Build storage systems in Rust.</p>".into(),
            ),
            employment_type: Some(EmploymentType::FullTime),
            external_id: "F4C096B22E".into(),
            locations: vec![JobLocation {
              name: "Paris, Ile-de-France, France".into(),
            }],
            published_at: Some(
              DateTime::parse_from_rfc3339("2026-07-30T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            ),
            raw: response["jobs"][0].clone(),
            title: "Senior Software Engineer".into(),
            workplace: Some(Workplace::Remote),
          },
          JobDraft {
            apply_url: Url::parse(
              "https://apply.workable.com/j/A1B2C3D4E5/apply",
            )
            .unwrap(),
            description_html: None,
            employment_type: None,
            external_id: "A1B2C3D4E5".into(),
            locations: vec![JobLocation {
              name: "London, England, UK".into(),
            }],
            published_at: None,
            raw: response["jobs"][1].clone(),
            title: "General Application".into(),
            workplace: Some(Workplace::OnSite),
          },
        ],
      },
    );
  }

  #[test]
  fn merges_duplicate_jobs_across_locations() {
    let snapshot = Workable::new("acme")
      .normalize(
        br#"{
          "jobs": [
            {
              "application_url": "https://apply.workable.com/j/ABC/apply",
              "locations": [{"city": "Paris", "country": "France"}],
              "shortcode": "ABC",
              "title": "Engineer"
            },
            {
              "application_url": "https://apply.workable.com/j/ABC/apply",
              "locations": [{"city": "Berlin", "country": "Germany"}],
              "shortcode": "ABC",
              "title": "Engineer"
            }
          ]
        }"#,
      )
      .unwrap();

    assert_eq!(snapshot.jobs.len(), 1);
    assert_eq!(
      snapshot.jobs[0].locations,
      vec![
        JobLocation {
          name: "Paris, France".into(),
        },
        JobLocation {
          name: "Berlin, Germany".into(),
        },
      ],
    );
  }
}
