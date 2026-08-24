use super::*;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "type")]
pub(crate) enum AdapterConfig {
  Ashby { board_name: String },
  Breezy { company_slug: String },
  Comeet { company_id: String, token: String },
  Greenhouse { board_token: String },
  Lever { site: String },
  Personio { account: String },
  Recruitee { company_slug: String },
  Teamtailor { company: String },
  Workable { account: String },
}

impl AdapterConfig {
  #[must_use]
  pub(crate) fn adapter(&self) -> (Box<dyn Adapter>, Value) {
    match self {
      Self::Ashby { board_name } => (
        Box::new(Ashby::new(board_name)),
        serde_json::json!({ "board_name": board_name }),
      ),
      Self::Breezy { company_slug } => (
        Box::new(Breezy::new(company_slug)),
        serde_json::json!({ "company_slug": company_slug }),
      ),
      Self::Comeet { company_id, token } => (
        Box::new(Comeet::new(company_id, token)),
        serde_json::json!({ "company_id": company_id }),
      ),
      Self::Greenhouse { board_token } => (
        Box::new(Greenhouse::new(board_token)),
        serde_json::json!({ "board_token": board_token }),
      ),
      Self::Lever { site } => (
        Box::new(Lever::new(site)),
        serde_json::json!({ "site": site }),
      ),
      Self::Personio { account } => (
        Box::new(Personio::new(account)),
        serde_json::json!({ "account": account }),
      ),
      Self::Recruitee { company_slug } => (
        Box::new(Recruitee::new(company_slug)),
        serde_json::json!({ "company_slug": company_slug }),
      ),
      Self::Teamtailor { company } => (
        Box::new(Teamtailor::new(company)),
        serde_json::json!({ "company": company }),
      ),
      Self::Workable { account } => (
        Box::new(Workable::new(account)),
        serde_json::json!({ "account": account }),
      ),
    }
  }

  #[must_use]
  pub(crate) const fn kind(&self) -> &'static str {
    match self {
      Self::Ashby { .. } => "ashby",
      Self::Breezy { .. } => "breezy",
      Self::Comeet { .. } => "comeet",
      Self::Greenhouse { .. } => "greenhouse",
      Self::Lever { .. } => "lever",
      Self::Personio { .. } => "personio",
      Self::Recruitee { .. } => "recruitee",
      Self::Teamtailor { .. } => "teamtailor",
      Self::Workable { .. } => "workable",
    }
  }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Config {
  pub sources: Vec<SourceConfig>,
}

impl Config {
  /// Parses application configuration from TOML.
  ///
  /// # Errors
  ///
  /// Returns an error when the input is not valid TOML or does not match the
  /// expected configuration shape.
  pub(crate) fn from_toml(input: &str) -> Result<Self, toml::de::Error> {
    toml::from_str(input)
  }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct SourceConfig {
  pub adapter: AdapterConfig,
  #[serde(default = "enabled_by_default")]
  pub enabled: bool,
  pub id: String,
  pub organization: String,
}

const fn enabled_by_default() -> bool {
  true
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn enabled_defaults_to_true() {
    let config = Config::from_toml(
      r#"
        [[sources]]
        id = "acme-careers"
        organization = "Acme"

        [sources.adapter]
        type = "greenhouse"
        board_token = "acme"
      "#,
    )
    .unwrap();

    assert!(config.sources[0].enabled);
  }

  #[test]
  fn parses_repository_configuration() {
    let config = Config::from_toml(include_str!("../config.toml")).unwrap();

    assert!(!config.sources.is_empty());
    assert!(config.sources.iter().all(|source| {
      !source.id.trim().is_empty() && !source.organization.trim().is_empty()
    }));

    let ids = config
      .sources
      .iter()
      .map(|source| &source.id)
      .collect::<HashSet<_>>();

    assert_eq!(ids.len(), config.sources.len());
  }

  #[test]
  fn rejects_unknown_adapter_fields() {
    let error = Config::from_toml(
      r#"
        [[sources]]
        id = "acme-careers"
        organization = "Acme"

        [sources.adapter]
        type = "greenhouse"
        board_tokne = "acme"
      "#,
    )
    .unwrap_err();

    assert!(error.to_string().contains("board_tokne"));
  }
}
