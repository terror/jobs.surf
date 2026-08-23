use super::*;

#[derive(Args, Clone, Debug, Eq, PartialEq)]
pub(crate) struct Options {
  #[arg(
    default_value = "postgres://jobs_surf:jobs_surf@localhost:5432/jobs_surf",
    env = "DATABASE_URL",
    global = true,
    long
  )]
  pub(crate) db_url: String,
}
