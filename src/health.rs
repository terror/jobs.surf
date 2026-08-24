use super::*;

/// Reports whether the service and its database are healthy.
#[utoipa::path(
  get,
  path = "/healthz",
  responses(
    (status = 200, description = "Service is healthy", body = String),
    (status = 503, description = "Database is unavailable"),
  ),
  tag = "health",
)]
pub(super) async fn get_health(
  AppState(state): AppState<State>,
) -> Result<&'static str, StatusCode> {
  state.db.ping().await.map_err(|error| {
    error!(%error, "database health check failed");
    StatusCode::SERVICE_UNAVAILABLE
  })?;

  Ok("ok")
}
