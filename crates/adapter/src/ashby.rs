use super::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("failed to decode jobs for Ashby board `{board_name}`")]
  Decode {
    board_name: String,
    #[source]
    source: serde_json::Error,
  },
}

#[derive(Deserialize)]
enum ProviderEmploymentType {
  Contract,
  FullTime,
  Intern,
  PartTime,
  Temporary,
}

#[derive(Deserialize)]
enum ProviderWorkplace {
  Hybrid,
  OnSite,
  Remote,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderJob {
  apply_url: Url,
  description_html: Option<String>,
  employment_type: Option<ProviderEmploymentType>,
  id: String,
  is_listed: Option<bool>,
  is_remote: Option<bool>,
  location: Option<String>,
  published_at: Option<DateTime<Utc>>,
  #[serde(default)]
  secondary_locations: Vec<SecondaryLocation>,
  title: String,
  workplace_type: Option<ProviderWorkplace>,
}

#[derive(Deserialize)]
struct Response {
  jobs: Vec<Value>,
}

#[derive(Deserialize)]
struct SecondaryLocation {
  location: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ashby {
  board_name: String,
}

impl Ashby {
  #[must_use]
  pub fn board_name(&self) -> &str {
    &self.board_name
  }

  #[must_use]
  pub fn new(board_name: impl Into<String>) -> Self {
    Self {
      board_name: board_name.into(),
    }
  }
}

impl Adapter for Ashby {
  fn normalize(&self, response: &[u8]) -> Result<JobSnapshot> {
    let response: Response =
      serde_json::from_slice(response).map_err(|source| Error::Decode {
        board_name: self.board_name.clone(),
        source,
      })?;

    let jobs = response
      .jobs
      .into_iter()
      .map(|raw| {
        let job: ProviderJob =
          serde_json::from_value(raw.clone()).map_err(|source| {
            Error::Decode {
              board_name: self.board_name.clone(),
              source,
            }
          })?;

        if job.is_listed == Some(false) {
          return Ok(None);
        }

        let employment_type =
          job
            .employment_type
            .map(|employment_type| match employment_type {
              ProviderEmploymentType::Contract => EmploymentType::Contract,
              ProviderEmploymentType::FullTime => EmploymentType::FullTime,
              ProviderEmploymentType::Intern => EmploymentType::Internship,
              ProviderEmploymentType::PartTime => EmploymentType::PartTime,
              ProviderEmploymentType::Temporary => EmploymentType::Temporary,
            });

        let locations = job
          .location
          .into_iter()
          .chain(
            job
              .secondary_locations
              .into_iter()
              .map(|location| location.location),
          )
          .filter(|location| !location.trim().is_empty())
          .map(|name| JobLocation { name })
          .collect();

        let workplace = match (job.workplace_type, job.is_remote) {
          (Some(ProviderWorkplace::Hybrid), _) => Some(Workplace::Hybrid),
          (Some(ProviderWorkplace::OnSite), _) => Some(Workplace::OnSite),
          (Some(ProviderWorkplace::Remote), _) | (None, Some(true)) => {
            Some(Workplace::Remote)
          }
          (None, Some(false) | None) => None,
        };

        Ok(Some(JobDraft {
          apply_url: job.apply_url,
          description_html: job.description_html,
          employment_type,
          external_id: job.id,
          locations,
          published_at: job.published_at,
          raw,
          title: job.title,
          workplace,
        }))
      })
      .collect::<Result<Vec<_>, Error>>()?
      .into_iter()
      .flatten()
      .collect();

    Ok(JobSnapshot { jobs })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const FIXTURE: &[u8] = include_bytes!("../tests/fixtures/ashby/jobs.json");

  #[test]
  fn normalizes_jobs() {
    let adapter = Ashby::new("acme");

    let response: Value = serde_json::from_slice(FIXTURE).unwrap();

    assert_eq!(
      adapter.normalize(FIXTURE).unwrap(),
      JobSnapshot {
        jobs: vec![
          JobDraft {
            apply_url: Url::parse(
              "https://jobs.ashbyhq.com/acme/7458d4e9-da2e-47bd-98cb-adfda43d42b2/application",
            )
            .unwrap(),
            description_html: Some(
              "<p>Build reliable systems &amp; tools.</p>".into(),
            ),
            employment_type: Some(EmploymentType::FullTime),
            external_id: "7458d4e9-da2e-47bd-98cb-adfda43d42b2".into(),
            locations: vec![
              JobLocation {
                name: "Remote - European Union".into(),
              },
              JobLocation {
                name: "Spain".into(),
              },
              JobLocation {
                name: "Germany".into(),
              },
            ],
            published_at: Some(
              DateTime::parse_from_rfc3339("2024-03-04T14:29:08.532+00:00",)
                .unwrap()
                .with_timezone(&Utc),
            ),
            raw: response["jobs"][0].clone(),
            title: "Engineering Manager - EU".into(),
            workplace: Some(Workplace::Remote),
          },
          JobDraft {
            apply_url: Url::parse(
              "https://jobs.ashbyhq.com/acme/c1897f22-5919-45a7-876a-d57f432ac93a/application",
            )
            .unwrap(),
            description_html: None,
            employment_type: Some(EmploymentType::Internship),
            external_id: "c1897f22-5919-45a7-876a-d57f432ac93a".into(),
            locations: Vec::new(),
            published_at: None,
            raw: response["jobs"][1].clone(),
            title: "Design Intern".into(),
            workplace: Some(Workplace::Hybrid),
          },
        ],
      },
    );
  }
}
