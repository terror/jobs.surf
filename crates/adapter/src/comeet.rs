use super::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("failed to decode jobs for Comeet company `{company_id}`")]
  Decode {
    company_id: String,
    #[source]
    source: serde_json::Error,
  },
  #[error(
    "Comeet job `{external_id}` for company `{company_id}` has no public URL"
  )]
  MissingApplyUrl {
    company_id: String,
    external_id: String,
  },
}

#[derive(Deserialize)]
enum ProviderWorkplace {
  Hybrid,
  #[serde(rename = "On-site")]
  OnSite,
  Remote,
}

#[derive(Deserialize)]
struct ProviderDetail {
  order: i64,
  value: String,
}

#[derive(Deserialize)]
struct ProviderLocation {
  city: Option<String>,
  country: Option<String>,
  is_remote: Option<bool>,
  name: Option<String>,
  state: Option<String>,
}

#[derive(Deserialize)]
struct ProviderJob {
  #[serde(default)]
  details: Vec<ProviderDetail>,
  employment_type: Option<String>,
  location: Option<ProviderLocation>,
  name: String,
  uid: String,
  url_active_page: Option<Url>,
  url_comeet_hosted_page: Option<Url>,
  workplace_type: Option<ProviderWorkplace>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Comeet {
  company_id: String,
}

impl Comeet {
  #[must_use]
  pub fn company_id(&self) -> &str {
    &self.company_id
  }

  #[must_use]
  pub fn new(company_id: impl Into<String>) -> Self {
    Self {
      company_id: company_id.into(),
    }
  }
}

impl Adapter for Comeet {
  fn normalize(&self, response: &[u8]) -> Result<JobSnapshot> {
    let response: Vec<Value> =
      serde_json::from_slice(response).map_err(|source| Error::Decode {
        company_id: self.company_id.clone(),
        source,
      })?;

    let jobs = response
      .into_iter()
      .map(|raw| {
        let mut job: ProviderJob = serde_json::from_value(raw.clone())
          .map_err(|source| Error::Decode {
            company_id: self.company_id.clone(),
            source,
          })?;

        let apply_url = job
          .url_active_page
          .or(job.url_comeet_hosted_page)
          .ok_or_else(|| Error::MissingApplyUrl {
            company_id: self.company_id.clone(),
            external_id: job.uid.clone(),
          })?;

        job.details.sort_by_key(|detail| detail.order);

        let description_html = {
          let description = job
            .details
            .into_iter()
            .filter(|detail| !detail.value.is_empty())
            .map(|detail| detail.value)
            .collect::<String>();

          (!description.is_empty()).then_some(description)
        };

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

        let location_is_remote = job
          .location
          .as_ref()
          .and_then(|location| location.is_remote);

        let locations = job
          .location
          .and_then(|location| {
            location
              .name
              .filter(|name| !name.trim().is_empty())
              .or_else(|| {
                let name = [location.city, location.state, location.country]
                  .into_iter()
                  .flatten()
                  .filter(|part| !part.trim().is_empty())
                  .collect::<Vec<_>>()
                  .join(", ");

                (!name.is_empty()).then_some(name)
              })
          })
          .map(|name| JobLocation { name })
          .into_iter()
          .collect();

        let workplace = match (job.workplace_type, location_is_remote) {
          (Some(ProviderWorkplace::Hybrid), _) => Some(Workplace::Hybrid),
          (Some(ProviderWorkplace::OnSite), _) => Some(Workplace::OnSite),
          (Some(ProviderWorkplace::Remote), _) | (None, Some(true)) => {
            Some(Workplace::Remote)
          }
          (None, Some(false) | None) => None,
        };

        Ok(JobDraft {
          apply_url,
          description_html,
          employment_type,
          external_id: job.uid,
          locations,
          published_at: None,
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

  const FIXTURE: &[u8] = include_bytes!("../tests/fixtures/comeet/jobs.json");

  #[test]
  fn normalizes_jobs() {
    let adapter = Comeet::new("E5.007");

    let response: Value = serde_json::from_slice(FIXTURE).unwrap();

    assert_eq!(
      adapter.normalize(FIXTURE).unwrap(),
      JobSnapshot {
        jobs: vec![
          JobDraft {
            apply_url: Url::parse(
              "https://www.comeet.com/jobs/acme/E5.007/android-developer/E8.91F",
            )
            .unwrap(),
            description_html: Some(
              "<p>Build reliable mobile systems.</p><p>Know Rust and Kotlin.</p>"
                .into(),
            ),
            employment_type: Some(EmploymentType::FullTime),
            external_id: "E8.91F".into(),
            locations: vec![JobLocation {
              name: "New York".into(),
            }],
            published_at: None,
            raw: response[0].clone(),
            title: "Android Developer".into(),
            workplace: Some(Workplace::OnSite),
          },
          JobDraft {
            apply_url: Url::parse(
              "https://www.comeet.com/jobs/acme/E5.007/platform-engineer/A2.123",
            )
            .unwrap(),
            description_html: None,
            employment_type: Some(EmploymentType::Contract),
            external_id: "A2.123".into(),
            locations: vec![JobLocation {
              name: "London, England, UK".into(),
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
