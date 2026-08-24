use super::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("failed to decode jobs for Recruitee company `{company_slug}`")]
  Decode {
    company_slug: String,
    #[source]
    source: serde_json::Error,
  },
  #[error(
    "Recruitee job `{external_id}` for company `{company_slug}` has an invalid publication date"
  )]
  InvalidPublishedAt {
    company_slug: String,
    external_id: String,
    #[source]
    source: chrono::ParseError,
  },
}

#[derive(Deserialize)]
struct ProviderLocation {
  city: Option<String>,
  country: Option<String>,
  name: Option<String>,
  state: Option<String>,
}

#[derive(Deserialize)]
struct ProviderJob {
  careers_apply_url: Url,
  description: Option<String>,
  employment_type_code: Option<String>,
  hybrid: Option<bool>,
  id: u64,
  location: Option<String>,
  #[serde(default)]
  locations: Vec<ProviderLocation>,
  on_site: Option<bool>,
  published_at: Option<String>,
  remote: Option<bool>,
  requirements: Option<String>,
  title: String,
}

#[derive(Deserialize)]
struct Response {
  offers: Vec<Value>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Recruitee {
  company_slug: String,
}

impl Recruitee {
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

  fn normalize_published_at(
    &self,
    external_id: &str,
    published_at: Option<String>,
  ) -> Result<Option<DateTime<Utc>>, Error> {
    published_at
      .map(|published_at| {
        NaiveDateTime::parse_from_str(&published_at, "%Y-%m-%d %H:%M:%S UTC")
          .map(|published_at| published_at.and_utc())
          .map_err(|source| Error::InvalidPublishedAt {
            company_slug: self.company_slug.clone(),
            external_id: external_id.into(),
            source,
          })
      })
      .transpose()
  }
}

#[async_trait::async_trait]
impl Adapter for Recruitee {
  async fn fetch(&self) -> Result<JobSnapshot> {
    let client = reqwest::Client::new();

    let url = http::parse_url(
      "Recruitee",
      &self.company_slug,
      format!("https://{}.recruitee.com/api/offers/", self.company_slug),
    )?;

    let response =
      http::get(&client, "Recruitee", &self.company_slug, url).await?;

    self.normalize(&response)
  }

  fn normalize(&self, response: &[u8]) -> Result<JobSnapshot> {
    let response: Response =
      serde_json::from_slice(response).map_err(|source| Error::Decode {
        company_slug: self.company_slug.clone(),
        source,
      })?;

    let jobs = response
      .offers
      .into_iter()
      .map(|raw| {
        let job: ProviderJob =
          serde_json::from_value(raw.clone()).map_err(|source| {
            Error::Decode {
              company_slug: self.company_slug.clone(),
              source,
            }
          })?;

        let external_id = job.id.to_string();

        let description_html = [job.description, job.requirements]
          .into_iter()
          .flatten()
          .filter(|description| !description.is_empty())
          .collect::<String>();
        let description_html =
          (!description_html.is_empty()).then_some(description_html);

        let employment_type = match job.employment_type_code.as_deref() {
          Some("contract" | "freelance") => Some(EmploymentType::Contract),
          Some("fulltime" | "fulltime_fixed_term" | "fulltime_permanent") => {
            Some(EmploymentType::FullTime)
          }
          Some("apprenticeship" | "internship") => {
            Some(EmploymentType::Internship)
          }
          Some(
            "parttime"
            | "parttime_fixed_term"
            | "parttime_minijob"
            | "parttime_permanent",
          ) => Some(EmploymentType::PartTime),
          Some("seasonal" | "temporary") => Some(EmploymentType::Temporary),
          Some(_) | None => None,
        };

        let locations: Vec<String> = if job.locations.is_empty() {
          job
            .location
            .filter(|location| !location.trim().is_empty())
            .into_iter()
            .collect()
        } else {
          job
            .locations
            .into_iter()
            .filter_map(|location| {
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
            .collect()
        };

        let locations = locations
          .into_iter()
          .map(|name| JobLocation { name })
          .collect();

        let published_at =
          self.normalize_published_at(&external_id, job.published_at)?;

        let workplace = match (job.remote, job.hybrid, job.on_site) {
          (Some(true), _, _) => Some(Workplace::Remote),
          (_, Some(true), _) => Some(Workplace::Hybrid),
          (_, _, Some(true)) => Some(Workplace::OnSite),
          _ => None,
        };

        Ok(JobDraft {
          apply_url: job.careers_apply_url,
          description_html,
          employment_type,
          external_id,
          locations,
          published_at,
          raw,
          title: job.title,
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
    include_bytes!("../tests/fixtures/recruitee/jobs.json");

  #[test]
  fn normalizes_jobs() {
    let adapter = Recruitee::new("acme");

    let response: Value = serde_json::from_slice(FIXTURE).unwrap();

    assert_eq!(
      adapter.normalize(FIXTURE).unwrap(),
      JobSnapshot {
        jobs: vec![
          JobDraft {
            apply_url: Url::parse(
              "https://jobs.acme.com/o/product-manager/c/new",
            )
            .unwrap(),
            description_html: Some(
              "<p>Build the product.</p><h4>What you bring</h4>".into(),
            ),
            employment_type: Some(EmploymentType::FullTime),
            external_id: "2715078".into(),
            locations: vec![JobLocation {
              name: "Utrecht".into(),
            }],
            published_at: Some(
              DateTime::parse_from_rfc3339("2026-08-19T10:48:22Z")
                .unwrap()
                .with_timezone(&Utc),
            ),
            raw: response["offers"][0].clone(),
            title: "Product Manager".into(),
            workplace: Some(Workplace::Remote),
          },
          JobDraft {
            apply_url: Url::parse(
              "https://jobs.acme.com/o/platform-intern/c/new",
            )
            .unwrap(),
            description_html: None,
            employment_type: Some(EmploymentType::Internship),
            external_id: "2715079".into(),
            locations: vec![JobLocation {
              name: "London, UK".into(),
            }],
            published_at: None,
            raw: response["offers"][1].clone(),
            title: "Platform Intern".into(),
            workplace: Some(Workplace::OnSite),
          },
        ],
      },
    );
  }
}
