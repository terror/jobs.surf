use super::*;

#[derive(OpenApi)]
#[openapi(
  info(
    description = "Read-only API for aggregated job listings.",
    title = "jobs.surf API",
  ),
  paths(health::get_health, jobs::get_jobs),
  tags(
    (name = "health", description = "Service health"),
    (name = "jobs", description = "Job listings"),
  ),
)]
pub(crate) struct Documentation;
