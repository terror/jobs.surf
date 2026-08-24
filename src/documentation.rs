use super::*;

#[derive(OpenApi)]
#[openapi(
  info(
    description = "Read-only API for aggregated job listings.",
    title = "jobs.surf API",
  ),
  paths(
    health::get_health,
    jobs::get_job,
    jobs::get_jobs,
    sources::get_sources,
  ),
  tags(
    (name = "health", description = "Service health"),
    (name = "jobs", description = "Job listings"),
    (name = "sources", description = "Job sources"),
  ),
)]
pub(crate) struct Documentation;
