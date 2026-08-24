use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JobPage {
  pub jobs: Vec<JobRecord>,
  pub next_cursor: Option<JobCursor>,
}
