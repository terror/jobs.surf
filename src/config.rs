use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "type")]
pub enum AdapterConfig {
  Ashby { board_name: String },
  Breezy { company_slug: String },
  Comeet { company_id: String },
  Greenhouse { board_token: String },
  Lever { site: String },
  Personio { account: String },
  Recruitee { company_slug: String },
  Workable { account: String },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config {
  pub sources: Vec<SourceConfig>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SourceConfig {
  pub adapter: AdapterConfig,
  #[serde(default = "enabled_by_default")]
  pub enabled: bool,
  pub id: String,
  pub organization: String,
}

impl Config {
  /// Parses application configuration from TOML.
  ///
  /// # Errors
  ///
  /// Returns an error when the input is not valid TOML or does not match the
  /// expected configuration shape.
  pub fn from_toml(input: &str) -> Result<Self, toml::de::Error> {
    toml::from_str(input)
  }
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
  fn parses_example() {
    let config = Config::from_toml(include_str!("../config.toml")).unwrap();

    assert_eq!(
      config,
      Config {
        sources: vec![SourceConfig {
          adapter: AdapterConfig::Greenhouse {
            board_token: "acme".into(),
          },
          enabled: true,
          id: "acme-careers".into(),
          organization: "Acme".into(),
        }],
      },
    );
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
