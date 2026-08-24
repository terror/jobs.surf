use super::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("failed to decode jobs for Greenhouse board `{board_token}`")]
  Decode {
    board_token: String,
    #[source]
    source: serde_json::Error,
  },
}

#[derive(Deserialize)]
struct Location {
  name: String,
}

#[derive(Deserialize)]
struct ProviderJob {
  absolute_url: Url,
  content: Option<String>,
  id: u64,
  location: Option<Location>,
  title: String,
}

#[derive(Deserialize)]
struct Response {
  jobs: Vec<Value>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Greenhouse {
  board_token: String,
}

impl Greenhouse {
  #[must_use]
  pub fn board_token(&self) -> &str {
    &self.board_token
  }

  #[must_use]
  pub fn new(board_token: impl Into<String>) -> Self {
    Self {
      board_token: board_token.into(),
    }
  }
}

impl Adapter for Greenhouse {
  fn normalize(&self, response: &[u8]) -> Result<JobSnapshot> {
    let response: Response =
      serde_json::from_slice(response).map_err(|source| Error::Decode {
        board_token: self.board_token.clone(),
        source,
      })?;

    let jobs = response
      .jobs
      .into_iter()
      .map(|raw| {
        let job: ProviderJob =
          serde_json::from_value(raw.clone()).map_err(|source| {
            Error::Decode {
              board_token: self.board_token.clone(),
              source,
            }
          })?;

        let locations = job
          .location
          .map(|location| JobLocation {
            name: location.name,
          })
          .into_iter()
          .collect();

        Ok(JobDraft {
          apply_url: job.absolute_url,
          description_html: job
            .content
            .map(|content| decode_html_entities(&content).into_owned()),
          employment_type: None,
          external_id: job.id.to_string(),
          locations,
          published_at: None,
          raw,
          title: job.title,
          workplace: None,
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
    include_bytes!("../tests/fixtures/greenhouse/jobs.json");

  #[test]
  fn normalizes_jobs() {
    let adapter = Greenhouse::new("acme");

    let response: Value = serde_json::from_slice(FIXTURE).unwrap();

    assert_eq!(
      adapter.normalize(FIXTURE).unwrap(),
      JobSnapshot {
        jobs: vec![
          JobDraft {
            apply_url: Url::parse(
              "https://boards.greenhouse.io/acme/jobs/127817",
            )
            .unwrap(),
            description_html: Some(
              "<p>Build reliable systems & tools.</p>".into(),
            ),
            employment_type: None,
            external_id: "127817".into(),
            locations: vec![JobLocation {
              name: "New York, NY".into(),
            }],
            published_at: None,
            raw: response["jobs"][0].clone(),
            title: "Rust Engineer".into(),
            workplace: None,
          },
          JobDraft {
            apply_url: Url::parse(
              "https://boards.greenhouse.io/acme/jobs/127818",
            )
            .unwrap(),
            description_html: None,
            employment_type: None,
            external_id: "127818".into(),
            locations: Vec::new(),
            published_at: None,
            raw: response["jobs"][1].clone(),
            title: "General Application".into(),
            workplace: None,
          },
        ],
      },
    );
  }
}
