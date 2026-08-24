use super::*;

const DEFAULT_LIMIT: u16 = 20;
const MAX_LIMIT: u16 = 100;

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
enum Error {
  #[error("failed to encode pagination cursor")]
  CursorEncoding(#[source] serde_json::Error),
  #[error("invalid pagination cursor")]
  InvalidCursor,
  #[error("invalid pagination limit")]
  InvalidLimit,
  #[error(transparent)]
  Repository(#[from] jobs_surf_db::Error),
}

impl IntoResponse for Error {
  fn into_response(self) -> Response {
    let (status, message) = match self {
      Self::InvalidCursor => (StatusCode::BAD_REQUEST, "invalid cursor"),
      Self::InvalidLimit => {
        (StatusCode::BAD_REQUEST, "limit must be between 1 and 100")
      }
      Self::CursorEncoding(_) | Self::Repository(_) => {
        error!(%self, "failed to list jobs");
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

#[derive(Serialize)]
struct ErrorResponse {
  error: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JobResponse {
  apply_url: String,
  description_html: Option<String>,
  employment_type: Option<String>,
  id: String,
  locations: Vec<LocationResponse>,
  published_at: Option<DateTime<Utc>>,
  source_id: String,
  title: String,
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct JobsQuery {
  cursor: Option<String>,
  limit: Option<u16>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JobsResponse {
  jobs: Vec<JobResponse>,
  next_cursor: Option<String>,
}

#[derive(Serialize)]
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

pub(super) fn route() -> MethodRouter<State> {
  get(get_jobs)
}

async fn get_jobs(
  AppState(state): AppState<State>,
  Query(query): Query<JobsQuery>,
) -> Result<Json<JobsResponse>> {
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

  let page = state.db.list_jobs(cursor, limit).await?;

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
