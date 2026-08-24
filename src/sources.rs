use super::*;

pub(super) type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub(super) enum Error {
  #[error(transparent)]
  Repository(#[from] jobs_surf_db::Error),
}

impl IntoResponse for Error {
  fn into_response(self) -> Response {
    error!(%self, "failed to read sources");

    (
      StatusCode::INTERNAL_SERVER_ERROR,
      Json(ErrorResponse {
        error: "internal server error",
      }),
    )
      .into_response()
  }
}

#[derive(Serialize, ToSchema)]
struct SourceResponse {
  /// Adapter used to synchronize this source.
  adapter: String,
  /// Stable source identifier.
  id: String,
  /// Organization represented by this source.
  organization: String,
}

impl From<SourceRecord> for SourceResponse {
  fn from(source: SourceRecord) -> Self {
    Self {
      adapter: source.adapter,
      id: source.id,
      organization: source.organization,
    }
  }
}

#[derive(Serialize, ToSchema)]
pub(super) struct SourcesResponse {
  sources: Vec<SourceResponse>,
}

/// Lists all sources known to the database.
#[utoipa::path(
  get,
  path = "/v1/sources",
  operation_id = "listSources",
  responses(
    (status = 200, description = "Known job sources", body = SourcesResponse),
    (
      status = 500,
      description = "Failed to list sources",
      body = ErrorResponse,
    ),
  ),
  tag = "sources",
)]
pub(super) async fn get_sources(
  AppState(state): AppState<State>,
) -> Result<Json<SourcesResponse>> {
  let sources = state.db.list_sources().await?;

  Ok(Json(SourcesResponse {
    sources: sources.into_iter().map(SourceResponse::from).collect(),
  }))
}
