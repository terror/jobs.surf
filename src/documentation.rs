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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn documents_the_read_api() {
    let document = serde_json::to_value(Documentation::openapi()).unwrap();
    let paths = document["paths"].as_object().unwrap();

    assert!(paths.contains_key("/healthz"));
    assert!(paths.contains_key("/v1/jobs"));
    assert!(paths.contains_key("/v1/jobs/{id}"));
    assert!(paths.contains_key("/v1/sources"));

    let parameters = paths["/v1/jobs"]["get"]["parameters"]
      .as_array()
      .unwrap()
      .iter()
      .map(|parameter| parameter["name"].as_str().unwrap())
      .collect::<Vec<_>>();

    assert_eq!(parameters, ["cursor", "limit", "query", "remote", "source"],);
    assert!(
      paths["/v1/jobs/{id}"]["get"]["responses"]
        .get("404")
        .is_some()
    );
  }
}
