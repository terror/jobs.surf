use super::*;

const DEFAULT_LIMIT: u16 = 20;
const MAX_LIMIT: u16 = 100;

pub(super) type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub(super) enum Error {
  #[error("failed to encode pagination cursor")]
  CursorEncoding(#[source] serde_json::Error),
  #[error("invalid pagination cursor")]
  InvalidCursor,
  #[error("invalid job id")]
  InvalidJobId,
  #[error("invalid pagination limit")]
  InvalidLimit,
  #[error("invalid query parameters")]
  InvalidQuery,
  #[error("job not found")]
  JobNotFound,
  #[error(transparent)]
  Repository(#[from] jobs_surf_db::Error),
}

impl IntoResponse for Error {
  fn into_response(self) -> Response {
    let (status, message) = match self {
      Self::InvalidCursor => (StatusCode::BAD_REQUEST, "invalid cursor"),
      Self::InvalidJobId => (StatusCode::BAD_REQUEST, "invalid job id"),
      Self::InvalidLimit => {
        (StatusCode::BAD_REQUEST, "limit must be between 1 and 100")
      }
      Self::InvalidQuery => {
        (StatusCode::BAD_REQUEST, "invalid query parameters")
      }
      Self::JobNotFound => (StatusCode::NOT_FOUND, "job not found"),
      Self::CursorEncoding(_) | Self::Repository(_) => {
        error!(%self, "failed to read jobs");
        (StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
      }
    };

    (status, Json(ErrorResponse { error: message })).into_response()
  }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Cursor {
  first_seen_at: DateTime<Utc>,
  id: i64,
}

impl From<Cursor> for JobCursor {
  fn from(cursor: Cursor) -> Self {
    Self {
      first_seen_at: cursor.first_seen_at,
      id: cursor.id,
    }
  }
}

impl From<JobCursor> for Cursor {
  fn from(cursor: JobCursor) -> Self {
    Self {
      first_seen_at: cursor.first_seen_at,
      id: cursor.id,
    }
  }
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct JobResponse {
  /// URL where candidates can apply for the job.
  apply_url: String,
  /// Sanitized HTML description supplied by the job source.
  description_html: Option<String>,
  /// Employment type, such as `full_time`, `part_time`, or `contract`.
  employment_type: Option<String>,
  /// Stable jobs.surf job identifier.
  id: String,
  locations: Vec<LocationResponse>,
  published_at: Option<DateTime<Utc>>,
  /// Identifier of the source that supplied the job.
  source_id: String,
  title: String,
  /// Workplace arrangement: `remote`, `hybrid`, or `on_site`.
  workplace: Option<String>,
}

impl From<JobRecord> for JobResponse {
  fn from(job: JobRecord) -> Self {
    Self {
      apply_url: job.apply_url,
      description_html: job.description_html.map(|html| clean(&html)),
      employment_type: job.employment_type,
      id: job.id.to_string(),
      locations: job
        .locations
        .into_iter()
        .map(LocationResponse::from)
        .collect(),
      published_at: job.published_at,
      source_id: job.source_id,
      title: job.title,
      workplace: job.workplace,
    }
  }
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
#[serde(deny_unknown_fields)]
pub(super) struct JobsQuery {
  /// Opaque cursor returned by the previous page.
  cursor: Option<String>,
  /// Number of jobs to return.
  #[param(default = 20, maximum = 100, minimum = 1)]
  limit: Option<u16>,
  /// Full-text search across job titles and descriptions.
  query: Option<String>,
  /// Whether to return only fully remote or only non-remote jobs.
  remote: Option<bool>,
  /// Exact source identifier to match.
  source: Option<String>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct JobsResponse {
  jobs: Vec<JobResponse>,
  /// Cursor for the next page, or `null` when there are no more jobs.
  next_cursor: Option<String>,
}

#[derive(Serialize, ToSchema)]
struct LocationResponse {
  name: String,
}

impl From<JobLocation> for LocationResponse {
  fn from(location: JobLocation) -> Self {
    Self {
      name: location.name,
    }
  }
}

/// Returns one open job by its jobs.surf identifier.
#[utoipa::path(
  get,
  path = "/v1/jobs/{id}",
  operation_id = "getJob",
  params(
    ("id" = String, Path, description = "Positive decimal job identifier"),
  ),
  responses(
    (status = 200, description = "Open job", body = JobResponse),
    (status = 400, description = "Invalid job identifier", body = ErrorResponse),
    (status = 404, description = "Job not found", body = ErrorResponse),
    (status = 500, description = "Failed to get job", body = ErrorResponse),
  ),
  tag = "jobs",
)]
pub(super) async fn get_job(
  AppState(state): AppState<State>,
  Path(id): Path<String>,
) -> Result<Json<JobResponse>> {
  let id = id
    .parse::<i64>()
    .ok()
    .filter(|id| *id > 0)
    .ok_or(Error::InvalidJobId)?;

  let job = state.db.get_job(id).await?.ok_or(Error::JobNotFound)?;

  Ok(Json(JobResponse::from(job)))
}

/// Lists open jobs in newest-first order.
///
/// Pass `nextCursor` from a response as `cursor` and repeat the same filters to
/// retrieve the next page.
#[utoipa::path(
  get,
  path = "/v1/jobs",
  operation_id = "listJobs",
  params(JobsQuery),
  responses(
    (status = 200, description = "Open jobs", body = JobsResponse),
    (
      status = 400,
      description = "Invalid query parameters",
      body = ErrorResponse,
    ),
    (
      status = 500,
      description = "Failed to list jobs",
      body = ErrorResponse,
    ),
  ),
  tag = "jobs",
)]
pub(super) async fn get_jobs(
  AppState(state): AppState<State>,
  query: std::result::Result<Query<JobsQuery>, QueryRejection>,
) -> Result<Json<JobsResponse>> {
  let Query(query) = query.map_err(|_| Error::InvalidQuery)?;
  let limit = query.limit.unwrap_or(DEFAULT_LIMIT);

  let limit = NonZeroU16::new(limit)
    .filter(|limit| limit.get() <= MAX_LIMIT)
    .ok_or(Error::InvalidLimit)?;

  let cursor = query
    .cursor
    .as_deref()
    .map(|value| {
      let bytes = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| Error::InvalidCursor)?;

      let cursor = serde_json::from_slice::<Cursor>(&bytes)
        .map_err(|_| Error::InvalidCursor)?;

      if cursor.id <= 0 {
        return Err(Error::InvalidCursor);
      }

      Ok(cursor.into())
    })
    .transpose()?;

  let normalized_query = query
    .query
    .as_deref()
    .map(str::trim)
    .filter(|query| !query.is_empty());

  let page = state
    .db
    .list_jobs(
      cursor,
      limit,
      normalized_query,
      query.remote,
      query.source.as_deref(),
    )
    .await?;

  Ok(Json(JobsResponse {
    jobs: page.jobs.into_iter().map(JobResponse::from).collect(),
    next_cursor: page
      .next_cursor
      .map(|cursor| -> Result<String> {
        let bytes = serde_json::to_vec(&Cursor::from(cursor))
          .map_err(Error::CursorEncoding)?;

        Ok(URL_SAFE_NO_PAD.encode(bytes))
      })
      .transpose()?,
  }))
}
