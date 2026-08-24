use super::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("failed to decode jobs for Lever site `{site}`")]
  Decode {
    site: String,
    #[source]
    source: serde_json::Error,
  },
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ProviderWorkplace {
  Hybrid,
  OnSite,
  Remote,
  Unspecified,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Categories {
  #[serde(default)]
  all_locations: Vec<String>,
  commitment: Option<String>,
  location: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderJob {
  additional: Option<String>,
  apply_url: Url,
  categories: Option<Categories>,
  description: Option<String>,
  id: String,
  #[serde(default)]
  lists: Vec<ProviderList>,
  text: String,
  workplace_type: Option<ProviderWorkplace>,
}

#[derive(Deserialize)]
struct ProviderList {
  content: String,
  text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Lever {
  site: String,
}

impl Lever {
  #[must_use]
  pub fn new(site: impl Into<String>) -> Self {
    Self { site: site.into() }
  }

  #[must_use]
  pub fn site(&self) -> &str {
    &self.site
  }
}

#[async_trait::async_trait]
impl Adapter for Lever {
  async fn fetch(&self) -> Result<JobSnapshot> {
    const PAGE_SIZE: usize = 100;

    let client = reqwest::Client::new();

    let mut jobs = Vec::new();
    let mut skip = 0;

    loop {
      let mut url = http::parse_url(
        "Lever",
        &self.site,
        format!("https://api.lever.co/v0/postings/{}", self.site),
      )?;

      url
        .query_pairs_mut()
        .append_pair("mode", "json")
        .append_pair("limit", &PAGE_SIZE.to_string())
        .append_pair("skip", &skip.to_string());

      let response = http::get(&client, "Lever", &self.site, url).await?;

      let page: Vec<Value> =
        serde_json::from_slice(&response).map_err(|source| Error::Decode {
          site: self.site.clone(),
          source,
        })?;

      let page_len = page.len();

      jobs.extend(page);

      if page_len < PAGE_SIZE {
        break;
      }

      skip += PAGE_SIZE;
    }

    let response =
      serde_json::to_vec(&jobs).map_err(|source| Error::Decode {
        site: self.site.clone(),
        source,
      })?;

    self.normalize(&response)
  }

  fn normalize(&self, response: &[u8]) -> Result<JobSnapshot> {
    let response: Vec<Value> =
      serde_json::from_slice(response).map_err(|source| Error::Decode {
        site: self.site.clone(),
        source,
      })?;

    let jobs = response
      .into_iter()
      .map(|raw| {
        let job: ProviderJob =
          serde_json::from_value(raw.clone()).map_err(|source| {
            Error::Decode {
              site: self.site.clone(),
              source,
            }
          })?;

        let Categories {
          all_locations,
          commitment,
          location,
        } = job.categories.unwrap_or_default();

        let employment_type = commitment.as_deref().and_then(|commitment| {
          match commitment.to_ascii_lowercase().as_str() {
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

        let location_names = if all_locations.is_empty() {
          location.into_iter().collect()
        } else {
          all_locations
        };

        let locations = location_names
          .into_iter()
          .filter(|location| !location.trim().is_empty())
          .map(|name| JobLocation { name })
          .collect();

        let mut description_parts = Vec::new();

        if let Some(description) = job
          .description
          .filter(|description| !description.is_empty())
        {
          description_parts.push(description);
        }

        description_parts.extend(job.lists.into_iter().map(|list| {
          format!(
            "<h3>{}</h3><ul>{}</ul>",
            encode_text(&list.text),
            list.content,
          )
        }));

        if let Some(additional) =
          job.additional.filter(|additional| !additional.is_empty())
        {
          description_parts.push(additional);
        }

        let description_html =
          (!description_parts.is_empty()).then(|| description_parts.concat());

        let workplace = match job.workplace_type {
          Some(ProviderWorkplace::Hybrid) => Some(Workplace::Hybrid),
          Some(ProviderWorkplace::OnSite) => Some(Workplace::OnSite),
          Some(ProviderWorkplace::Remote) => Some(Workplace::Remote),
          Some(ProviderWorkplace::Unspecified) | None => None,
        };

        Ok(JobDraft {
          apply_url: job.apply_url,
          description_html,
          employment_type,
          external_id: job.id,
          locations,
          published_at: None,
          raw,
          title: job.text,
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

  const FIXTURE: &[u8] = include_bytes!("../tests/fixtures/lever/jobs.json");

  #[test]
  fn normalizes_jobs() {
    let adapter = Lever::new("acme");

    let response: Value = serde_json::from_slice(FIXTURE).unwrap();

    assert_eq!(
      adapter.normalize(FIXTURE).unwrap(),
      JobSnapshot {
        jobs: vec![
          JobDraft {
            apply_url: Url::parse(
              "https://jobs.lever.co/acme/33538a2f-d27d-4a96-8f05-fa4b0e4d940e/apply",
            )
            .unwrap(),
            description_html: Some(
              "<div>Build reliable systems.</div><h3>Impact &amp; ownership</h3><ul><li>Own ingestion.</li></ul><div>Acme is an equal opportunity employer.</div>"
                .into(),
            ),
            employment_type: Some(EmploymentType::FullTime),
            external_id: "33538a2f-d27d-4a96-8f05-fa4b0e4d940e".into(),
            locations: vec![
              JobLocation {
                name: "New York, NY".into(),
              },
              JobLocation {
                name: "Remote - US".into(),
              },
            ],
            published_at: None,
            raw: response[0].clone(),
            title: "Senior Rust Engineer".into(),
            workplace: Some(Workplace::Hybrid),
          },
          JobDraft {
            apply_url: Url::parse(
              "https://jobs.lever.co/acme/b73e31f4-fb4e-4555-a915-61a09c10f7dd/apply",
            )
            .unwrap(),
            description_html: None,
            employment_type: None,
            external_id: "b73e31f4-fb4e-4555-a915-61a09c10f7dd".into(),
            locations: vec![JobLocation {
              name: "London, UK".into(),
            }],
            published_at: None,
            raw: response[1].clone(),
            title: "General Application".into(),
            workplace: None,
          },
        ],
      },
    );
  }
}
