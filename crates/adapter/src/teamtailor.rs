use super::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("failed to decode jobs for Teamtailor company `{company}`")]
  Decode {
    company: String,
    #[source]
    source: serde_json::Error,
  },
  #[error(
    "Teamtailor job `{external_id}` for company `{company}` has an invalid application URL"
  )]
  InvalidApplyUrl {
    company: String,
    external_id: String,
    #[source]
    source: url::ParseError,
  },
  #[error(
    "Teamtailor job `{external_id}` for company `{company}` has no application URL"
  )]
  MissingApplyUrl {
    company: String,
    external_id: String,
  },
  #[error(
    "Teamtailor job `{external_id}` for company `{company}` is missing location `{location_id}`"
  )]
  MissingLocation {
    company: String,
    external_id: String,
    location_id: String,
  },
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
struct ProviderAttributes {
  body: Option<String>,
  employment_type: Option<String>,
  external_application_url: Option<String>,
  remote_status: Option<String>,
  title: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
struct ProviderLinks {
  careersite_job_apply_url: Option<String>,
}

#[derive(Deserialize)]
struct ProviderLocation {
  attributes: ProviderLocationAttributes,
  id: String,
  #[serde(rename = "type")]
  kind: String,
}

#[derive(Deserialize)]
struct ProviderLocationAttributes {
  city: Option<String>,
  country: Option<String>,
  name: Option<String>,
}

#[derive(Deserialize)]
struct ProviderJob {
  attributes: ProviderAttributes,
  id: String,
  links: ProviderLinks,
  relationships: Option<ProviderRelationships>,
}

#[derive(Deserialize)]
struct ProviderRelationship {
  #[serde(default)]
  data: Vec<ProviderResourceIdentifier>,
}

#[derive(Deserialize)]
struct ProviderRelationships {
  locations: Option<ProviderRelationship>,
}

#[derive(Deserialize)]
struct ProviderResourceIdentifier {
  id: String,
  #[serde(rename = "type")]
  kind: String,
}

#[derive(Deserialize)]
struct Response {
  data: Vec<Value>,
  #[serde(default)]
  included: Vec<ProviderLocation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Teamtailor {
  company: String,
}

impl Teamtailor {
  fn apply_url(&self, job: &ProviderJob) -> Result<Url, Error> {
    let url = job
      .attributes
      .external_application_url
      .as_deref()
      .filter(|url| !url.trim().is_empty())
      .or(job.links.careersite_job_apply_url.as_deref())
      .ok_or_else(|| Error::MissingApplyUrl {
        company: self.company.clone(),
        external_id: job.id.clone(),
      })?;

    Url::parse(url).map_err(|source| Error::InvalidApplyUrl {
      company: self.company.clone(),
      external_id: job.id.clone(),
      source,
    })
  }

  #[must_use]
  pub fn company(&self) -> &str {
    &self.company
  }

  #[must_use]
  pub fn new(company: impl Into<String>) -> Self {
    Self {
      company: company.into(),
    }
  }
}

impl Adapter for Teamtailor {
  fn normalize(&self, response: &[u8]) -> Result<JobSnapshot> {
    let response: Response =
      serde_json::from_slice(response).map_err(|source| Error::Decode {
        company: self.company.clone(),
        source,
      })?;

    let included = response
      .included
      .into_iter()
      .map(|location| ((location.kind.clone(), location.id.clone()), location))
      .collect::<HashMap<_, _>>();

    let jobs = response
      .data
      .into_iter()
      .map(|raw| {
        let job: ProviderJob =
          serde_json::from_value(raw.clone()).map_err(|source| {
            Error::Decode {
              company: self.company.clone(),
              source,
            }
          })?;

        let apply_url = self.apply_url(&job)?;

        let employment_type = match job.attributes.employment_type.as_deref() {
          Some("contract") => Some(EmploymentType::Contract),
          Some("fully") => Some(EmploymentType::FullTime),
          Some("internship") => Some(EmploymentType::Internship),
          Some("part") => Some(EmploymentType::PartTime),
          Some("temporary") => Some(EmploymentType::Temporary),
          Some(_) | None => None,
        };

        let locations = job
          .relationships
          .and_then(|relationships| relationships.locations)
          .into_iter()
          .flat_map(|locations| locations.data)
          .map(|identifier| {
            let location = included
              .get(&(identifier.kind, identifier.id.clone()))
              .ok_or_else(|| Error::MissingLocation {
                company: self.company.clone(),
                external_id: job.id.clone(),
                location_id: identifier.id,
              })?;

            Ok(
              location
                .attributes
                .name
                .as_ref()
                .filter(|name| !name.trim().is_empty())
                .cloned()
                .or_else(|| {
                  let name = [
                    location.attributes.city.as_ref(),
                    location.attributes.country.as_ref(),
                  ]
                  .into_iter()
                  .flatten()
                  .filter(|part| !part.trim().is_empty())
                  .cloned()
                  .collect::<Vec<_>>()
                  .join(", ");

                  (!name.is_empty()).then_some(name)
                }),
            )
          })
          .collect::<Result<Vec<_>, Error>>()?
          .into_iter()
          .flatten()
          .map(|name| JobLocation { name })
          .collect();

        let workplace = match job.attributes.remote_status.as_deref() {
          Some("fully") => Some(Workplace::Remote),
          Some("hybrid") => Some(Workplace::Hybrid),
          Some("none") => Some(Workplace::OnSite),
          Some(_) | None => None,
        };

        Ok(JobDraft {
          apply_url,
          description_html: job
            .attributes
            .body
            .filter(|description| !description.is_empty()),
          employment_type,
          external_id: job.id,
          locations,
          published_at: None,
          raw,
          title: job.attributes.title,
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

  const FIXTURE: &[u8] =
    include_bytes!("../tests/fixtures/teamtailor/jobs.json");

  #[test]
  fn normalizes_jobs() {
    let adapter = Teamtailor::new("acme");

    let response: Value = serde_json::from_slice(FIXTURE).unwrap();

    assert_eq!(
      adapter.normalize(FIXTURE).unwrap(),
      JobSnapshot {
        jobs: vec![
          JobDraft {
            apply_url: Url::parse("https://careers.acme.com/jobs/101/apply")
              .unwrap(),
            description_html: Some(
              "<p>Build reliable recruiting systems.</p>".into(),
            ),
            employment_type: Some(EmploymentType::FullTime),
            external_id: "101".into(),
            locations: vec![
              JobLocation {
                name: "Stockholm HQ".into(),
              },
              JobLocation {
                name: "Berlin, Germany".into(),
              },
            ],
            published_at: None,
            raw: response["data"][0].clone(),
            title: "Senior Rust Engineer".into(),
            workplace: Some(Workplace::Remote),
          },
          JobDraft {
            apply_url: Url::parse(
              "https://jobs.teamtailor.com/acme/jobs/102/apply",
            )
            .unwrap(),
            description_html: None,
            employment_type: Some(EmploymentType::Contract),
            external_id: "102".into(),
            locations: vec![JobLocation {
              name: "London".into(),
            }],
            published_at: None,
            raw: response["data"][1].clone(),
            title: "Platform Engineer".into(),
            workplace: Some(Workplace::OnSite),
          },
        ],
      },
    );
  }
}
