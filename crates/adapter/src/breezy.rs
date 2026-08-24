use super::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("failed to decode jobs for Breezy company `{company_slug}`")]
  Decode {
    company_slug: String,
    #[source]
    source: serde_json::Error,
  },
}

#[derive(Deserialize)]
struct ProviderEmploymentType {
  id: String,
}

#[derive(Deserialize)]
struct ProviderLabel {
  name: Option<String>,
}

#[derive(Deserialize)]
struct ProviderLocation {
  city: Option<String>,
  country: Option<ProviderLabel>,
  is_remote: Option<bool>,
  name: Option<String>,
  remote_details: Option<ProviderRemoteDetails>,
  state: Option<ProviderLabel>,
}

#[derive(Deserialize)]
struct ProviderRemoteDetails {
  value: Option<String>,
}

#[derive(Deserialize)]
struct ProviderJob {
  description: Option<String>,
  #[serde(rename = "type")]
  employment_type: Option<ProviderEmploymentType>,
  id: String,
  location: Option<ProviderLocation>,
  #[serde(default)]
  locations: Vec<ProviderLocation>,
  name: String,
  published_date: Option<DateTime<Utc>>,
  url: Url,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Breezy {
  company_slug: String,
}

impl Breezy {
  #[must_use]
  pub fn company_slug(&self) -> &str {
    &self.company_slug
  }

  #[must_use]
  pub fn new(company_slug: impl Into<String>) -> Self {
    Self {
      company_slug: company_slug.into(),
    }
  }
}

impl Adapter for Breezy {
  fn normalize(&self, response: &[u8]) -> Result<JobSnapshot> {
    let response: Vec<Value> =
      serde_json::from_slice(response).map_err(|source| Error::Decode {
        company_slug: self.company_slug.clone(),
        source,
      })?;

    let jobs = response
      .into_iter()
      .map(|raw| {
        let job: ProviderJob =
          serde_json::from_value(raw.clone()).map_err(|source| {
            Error::Decode {
              company_slug: self.company_slug.clone(),
              source,
            }
          })?;

        let employment_type =
          job
            .employment_type
            .and_then(|employment_type| match employment_type.id.as_str() {
              "contract" => Some(EmploymentType::Contract),
              "fullTime" => Some(EmploymentType::FullTime),
              "partTime" => Some(EmploymentType::PartTime),
              "temporary" => Some(EmploymentType::Temporary),
              _ => None,
            });

        let provider_locations = if job.locations.is_empty() {
          job.location.into_iter().collect()
        } else {
          job.locations
        };

        let workplace = if provider_locations.iter().any(|location| {
          location
            .remote_details
            .as_ref()
            .and_then(|details| details.value.as_deref())
            == Some("hybrid")
        }) {
          Some(Workplace::Hybrid)
        } else if provider_locations.iter().any(|location| {
          matches!(
            location
              .remote_details
              .as_ref()
              .and_then(|details| details.value.as_deref()),
            Some("remote" | "remote-location")
          )
        }) || provider_locations.iter().any(|location| {
          location.remote_details.is_none() && location.is_remote == Some(true)
        }) {
          Some(Workplace::Remote)
        } else {
          None
        };

        let locations = provider_locations
          .into_iter()
          .filter_map(|location| {
            location
              .name
              .filter(|name| !name.trim().is_empty())
              .or_else(|| {
                let name = [
                  location.city,
                  location.state.and_then(|state| state.name),
                  location.country.and_then(|country| country.name),
                ]
                .into_iter()
                .flatten()
                .filter(|part| !part.trim().is_empty())
                .collect::<Vec<_>>()
                .join(", ");

                (!name.is_empty()).then_some(name)
              })
          })
          .map(|name| JobLocation { name })
          .collect();

        Ok(JobDraft {
          apply_url: job.url,
          description_html: job
            .description
            .filter(|description| !description.is_empty()),
          employment_type,
          external_id: job.id,
          locations,
          published_at: job.published_date,
          raw,
          title: job.name,
          workplace,
        })
      })
      .collect::<Result<Vec<_>, Error>>()?;

    Ok(JobSnapshot { jobs })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const FIXTURE: &[u8] = include_bytes!("../tests/fixtures/breezy/jobs.json");

  #[test]
  fn normalizes_jobs() {
    let adapter = Breezy::new("acme");

    let response: Value = serde_json::from_slice(FIXTURE).unwrap();

    assert_eq!(
      adapter.normalize(FIXTURE).unwrap(),
      JobSnapshot {
        jobs: vec![
          JobDraft {
            apply_url: Url::parse(
              "https://acme.breezy.hr/p/abc123/senior-rust-engineer",
            )
            .unwrap(),
            description_html: Some(
              "<p>Build reliable recruiting systems.</p>".into(),
            ),
            employment_type: Some(EmploymentType::FullTime),
            external_id: "abc123".into(),
            locations: vec![
              JobLocation {
                name: "New York, NY".into(),
              },
              JobLocation {
                name: "Remote - US".into(),
              },
            ],
            published_at: Some(
              DateTime::parse_from_rfc3339("2026-08-20T14:30:00Z")
                .unwrap()
                .with_timezone(&Utc),
            ),
            raw: response[0].clone(),
            title: "Senior Rust Engineer".into(),
            workplace: Some(Workplace::Hybrid),
          },
          JobDraft {
            apply_url: Url::parse(
              "https://acme.breezy.hr/p/def456/platform-engineer",
            )
            .unwrap(),
            description_html: None,
            employment_type: None,
            external_id: "def456".into(),
            locations: vec![JobLocation {
              name: "London, England, United Kingdom".into(),
            }],
            published_at: None,
            raw: response[1].clone(),
            title: "Platform Engineer".into(),
            workplace: Some(Workplace::Remote),
          },
        ],
      },
    );
  }
}
