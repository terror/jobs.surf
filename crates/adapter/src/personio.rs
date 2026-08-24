use super::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("failed to decode a job for Personio account `{account}`")]
  Decode {
    account: String,
    #[source]
    source: de::DeError,
  },
  #[error(
    "Personio job `{external_id}` for account `{account}` has an invalid public URL"
  )]
  InvalidApplyUrl {
    account: String,
    external_id: String,
    #[source]
    source: url::ParseError,
  },
  #[error("Personio account `{account}` returned non-UTF-8 XML")]
  InvalidUtf8 {
    account: String,
    #[source]
    source: str::Utf8Error,
  },
  #[error("failed to parse XML for Personio account `{account}`")]
  InvalidXml {
    account: String,
    #[source]
    source: roxmltree::Error,
  },
  #[error(
    "unexpected `{element}` element in Personio account `{account}` feed"
  )]
  UnexpectedElement { account: String, element: String },
  #[error("unexpected `{root}` root in Personio account `{account}` feed")]
  UnexpectedRoot { account: String, root: String },
}

#[derive(Default, Deserialize)]
struct ProviderAdditionalOffices {
  #[serde(default, rename = "office")]
  offices: Vec<String>,
}

#[derive(Default, Deserialize)]
struct ProviderDescription {
  #[serde(default)]
  name: String,
  #[serde(default)]
  value: String,
}

#[derive(Default, Deserialize)]
struct ProviderDescriptions {
  #[serde(default, rename = "jobDescription")]
  descriptions: Vec<ProviderDescription>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderJob {
  additional_offices: Option<ProviderAdditionalOffices>,
  employment_type: Option<String>,
  id: String,
  job_descriptions: Option<ProviderDescriptions>,
  name: String,
  office: Option<String>,
  schedule: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Personio {
  account: String,
}

impl Personio {
  #[must_use]
  pub fn account(&self) -> &str {
    &self.account
  }

  fn apply_url(&self, external_id: &str) -> Result<Url, Error> {
    let host = if self.account.contains('.') {
      self.account.clone()
    } else {
      format!("{}.jobs.personio.de", self.account)
    };

    Url::parse(&format!("https://{host}/job/{external_id}")).map_err(|source| {
      Error::InvalidApplyUrl {
        account: self.account.clone(),
        external_id: external_id.into(),
        source,
      }
    })
  }

  #[must_use]
  pub fn new(account: impl Into<String>) -> Self {
    Self {
      account: account.into(),
    }
  }
}

impl Adapter for Personio {
  fn normalize(&self, response: &[u8]) -> Result<JobSnapshot> {
    let response =
      str::from_utf8(response).map_err(|source| Error::InvalidUtf8 {
        account: self.account.clone(),
        source,
      })?;

    let document =
      Document::parse(response).map_err(|source| Error::InvalidXml {
        account: self.account.clone(),
        source,
      })?;

    let root = document.root_element();

    if root.tag_name().name() != "workzag-jobs" {
      return Err(
        Error::UnexpectedRoot {
          account: self.account.clone(),
          root: root.tag_name().name().into(),
        }
        .into(),
      );
    }

    let jobs = root
      .children()
      .filter(Node::is_element)
      .map(|node| {
        if node.tag_name().name() != "position" {
          return Err(Error::UnexpectedElement {
            account: self.account.clone(),
            element: node.tag_name().name().into(),
          });
        }

        let raw_xml = &response[node.range()];
        let job: ProviderJob =
          de::from_str(raw_xml).map_err(|source| Error::Decode {
            account: self.account.clone(),
            source,
          })?;

        let description_html = job
          .job_descriptions
          .unwrap_or_default()
          .descriptions
          .into_iter()
          .filter(|description| !description.value.is_empty())
          .map(|description| {
            if description.name.is_empty() {
              description.value
            } else {
              format!(
                "<h3>{}</h3>{}",
                encode_text(&description.name),
                description.value,
              )
            }
          })
          .collect::<String>();

        let description_html =
          (!description_html.is_empty()).then_some(description_html);

        let employment_type = match job.employment_type.as_deref() {
          Some("fixed-term" | "fixed_term" | "freelance") => {
            Some(EmploymentType::Contract)
          }
          Some("intern" | "internship" | "trainee" | "working_student") => {
            Some(EmploymentType::Internship)
          }
          Some("temporary") => Some(EmploymentType::Temporary),
          Some(_) | None => match job.schedule.as_deref() {
            Some("full-time") => Some(EmploymentType::FullTime),
            Some("part-time") => Some(EmploymentType::PartTime),
            Some(_) | None => None,
          },
        };

        let locations = job
          .office
          .into_iter()
          .chain(job.additional_offices.unwrap_or_default().offices)
          .filter(|office| !office.trim().is_empty())
          .map(|name| JobLocation { name })
          .collect();

        Ok(JobDraft {
          apply_url: self.apply_url(&job.id)?,
          description_html,
          employment_type,
          external_id: job.id,
          locations,
          published_at: None,
          raw: Value::String(raw_xml.into()),
          title: job.name,
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

  const FIXTURE: &[u8] = include_bytes!("../tests/fixtures/personio/jobs.xml");

  #[test]
  fn normalizes_jobs() {
    let adapter = Personio::new("acme");

    let response = str::from_utf8(FIXTURE).unwrap();
    let document = Document::parse(response).unwrap();
    let raw = document
      .root_element()
      .children()
      .filter(Node::is_element)
      .map(|node| Value::String(response[node.range()].into()))
      .collect::<Vec<_>>();

    assert_eq!(
      adapter.normalize(FIXTURE).unwrap(),
      JobSnapshot {
        jobs: vec![
          JobDraft {
            apply_url: Url::parse(
              "https://acme.jobs.personio.de/job/2401151",
            )
            .unwrap(),
            description_html: Some(
              "<h3>Your mission</h3><p>Build reliable products.</p><h3>Profile &amp; skills</h3><p>Know Rust.</p>"
                .into(),
            ),
            employment_type: Some(EmploymentType::FullTime),
            external_id: "2401151".into(),
            locations: vec![
              JobLocation {
                name: "Berlin".into(),
              },
              JobLocation {
                name: "Remote".into(),
              },
            ],
            published_at: None,
            raw: raw[0].clone(),
            title: "Technical Product Manager".into(),
            workplace: None,
          },
          JobDraft {
            apply_url: Url::parse(
              "https://acme.jobs.personio.de/job/2401152",
            )
            .unwrap(),
            description_html: None,
            employment_type: Some(EmploymentType::Contract),
            external_id: "2401152".into(),
            locations: vec![JobLocation {
              name: "Lisbon".into(),
            }],
            published_at: None,
            raw: raw[1].clone(),
            title: "Platform Engineer".into(),
            workplace: None,
          },
        ],
      },
    );
  }
}
