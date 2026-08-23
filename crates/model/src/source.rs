use super::*;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
  pub id: String,
  pub organization: String,
  pub adapter: String,
  pub configuration: Value,
  pub enabled: bool,
}
