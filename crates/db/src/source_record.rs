#[derive(Clone, Debug, Eq, PartialEq, sqlx::FromRow)]
pub struct SourceRecord {
  pub adapter: String,
  pub id: String,
  pub organization: String,
}
