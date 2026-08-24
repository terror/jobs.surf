use super::*;

pub(super) fn route() -> MethodRouter<State> {
  get(get_health)
}

async fn get_health(
  AppState(state): AppState<State>,
) -> Result<&'static str, StatusCode> {
  state.db.ping().await.map_err(|error| {
    error!(%error, "database health check failed");
    StatusCode::SERVICE_UNAVAILABLE
  })?;

  Ok("ok")
}
