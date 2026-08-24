use super::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("failed to decode jobs for Greenhouse board `{board_token}`")]
  Decode {
    board_token: String,
    #[source]
    source: serde_json::Error,
  },
  #[error(
    "Greenhouse board `{board_token}` returned {actual} jobs but reported {expected}"
  )]
  IncompleteSnapshot {
    actual: usize,
    board_token: String,
    expected: usize,
  },
  #[error("Greenhouse API origin `{api_origin}` cannot be used as a base URL")]
  InvalidApiOrigin { api_origin: Url },
}

#[derive(Deserialize)]
struct Location {
  name: String,
}

#[derive(Deserialize)]
struct Meta {
  total: usize,
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
  meta: Meta,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Greenhouse {
  api_origin: Option<Url>,
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
      api_origin: None,
      board_token: board_token.into(),
    }
  }

  /// Creates an adapter that fetches from a custom Greenhouse API origin.
  ///
  /// # Errors
  ///
  /// Returns an error if the URL cannot be used as a hierarchical base URL.
  pub fn with_api_origin(
    board_token: impl Into<String>,
    api_origin: Url,
  ) -> Result<Self> {
    if api_origin.cannot_be_a_base() {
      return Err(Error::InvalidApiOrigin { api_origin }.into());
    }

    Ok(Self {
      api_origin: Some(api_origin),
      board_token: board_token.into(),
    })
  }
}

#[async_trait::async_trait]
impl Adapter for Greenhouse {
  async fn fetch(&self) -> Result<JobSnapshot> {
    let client = reqwest::Client::new();

    let mut url = if let Some(api_origin) = &self.api_origin {
      api_origin.clone()
    } else {
      http::parse_url(
        "Greenhouse",
        &self.board_token,
        "https://boards-api.greenhouse.io".into(),
      )?
    };

    let api_origin = url.clone();
    url.set_fragment(None);
    url.set_path("/");
    url.set_query(None);
    url
      .path_segments_mut()
      .map_err(|()| Error::InvalidApiOrigin { api_origin })?
      .extend(["v1", "boards", &self.board_token, "jobs"]);

    url.query_pairs_mut().append_pair("content", "true");

    let response =
      http::get(&client, "Greenhouse", &self.board_token, url).await?;

    self.normalize(&response)
  }

  fn normalize(&self, response: &[u8]) -> Result<JobSnapshot> {
    let response: Response =
      serde_json::from_slice(response).map_err(|source| Error::Decode {
        board_token: self.board_token.clone(),
        source,
      })?;

    if response.jobs.len() != response.meta.total {
      return Err(
        Error::IncompleteSnapshot {
          actual: response.jobs.len(),
          board_token: self.board_token.clone(),
          expected: response.meta.total,
        }
        .into(),
      );
    }

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
