use super::*;

#[derive(Args, Debug)]
pub(crate) struct Serve {
  #[arg(long, default_value = "3000", help = "Port to listen on")]
  port: u16,
}

impl Serve {
  fn app(db: Db) -> Router {
    Router::new()
      .route("/healthz", get(health::get_health))
      .route("/v1/jobs", get(jobs::get_jobs))
      .with_state(State { db })
      .layer(TraceLayer::new_for_http())
  }

  pub(crate) async fn run(self, options: Options) -> Result {
    let db = Db::connect(&options.db_url).await?;

    let addr = SocketAddr::from(([0, 0, 0, 0], self.port));

    let listener = TcpListener::bind(addr)
      .await
      .with_context(|| format!("failed to bind to {addr}"))?;

    info!(%addr, "listening");

    axum::serve(listener, Self::app(db)).await?;

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  static TEST_DATABASE_NUMBER: AtomicUsize = AtomicUsize::new(0);

  struct Test {
    app: Router,
    db: Db,
  }

  impl Test {
    async fn get(&self, uri: &str) -> Response {
      self
        .app
        .clone()
        .oneshot(
          Request::builder()
            .method(Method::GET)
            .uri(uri)
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap()
    }

    async fn new() -> Self {
      let number = TEST_DATABASE_NUMBER.fetch_add(1, Ordering::Relaxed);

      let name = format!(
        "jobs-surf-test-{}-{}-{}",
        process::id(),
        SystemTime::now()
          .duration_since(UNIX_EPOCH)
          .unwrap()
          .as_millis(),
        number,
      );

      let url = format!("postgres://jobs_surf:jobs_surf@localhost:5432/{name}");

      Postgres::create_database(&url).await.unwrap();

      let db = Db::connect(&url).await.unwrap();

      Self {
        app: Serve::app(db.clone()),
        db,
      }
    }

    async fn sync(&self, jobs: Vec<JobDraft>) {
      let source = Source {
        adapter: "greenhouse".into(),
        enabled: true,
        id: "source".into(),
        organization: "Acme".into(),
        ..Default::default()
      };

      let sync_run_id = self.db.start_sync(&source).await.unwrap();

      self
        .db
        .complete_sync(sync_run_id, &source.id, &JobSnapshot { jobs })
        .await
        .unwrap();
    }
  }

  async fn response_json(response: Response) -> Value {
    serde_json::from_slice(
      &to_bytes(response.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap()
  }

  #[tokio::test]
  async fn health_route_reports_database_failure() {
    let test = Test::new().await;

    test.db.close().await;

    let response = test.get("/healthz").await;

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
  }

  #[tokio::test]
  async fn health_route_works() {
    let test = Test::new().await;

    let response = test.get("/healthz").await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
      to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
        .as_ref(),
      b"ok",
    );
  }

  #[tokio::test]
  async fn jobs_route_paginates_without_duplicates() {
    let test = Test::new().await;

    let first = JobDraft {
      apply_url: "https://example.com/jobs/one".parse().unwrap(),
      description_html: None,
      employment_type: None,
      external_id: "one".into(),
      locations: vec![JobLocation {
        name: "Remote".into(),
      }],
      published_at: None,
      raw: json!({ "private": true }),
      title: "First".into(),
      workplace: None,
    };

    let second = JobDraft {
      apply_url: "https://example.com/jobs/two".parse().unwrap(),
      external_id: "two".into(),
      title: "Second".into(),
      ..first.clone()
    };

    let third = JobDraft {
      apply_url: "https://example.com/jobs/three".parse().unwrap(),
      external_id: "three".into(),
      title: "Third".into(),
      ..first.clone()
    };

    test.sync(vec![first, second, third]).await;

    let response = test.get("/v1/jobs?limit=2").await;

    assert_eq!(response.status(), StatusCode::OK);

    let first_page = response_json(response).await;

    assert_eq!(
      first_page["jobs"],
      json!([
        {
          "applyUrl": "https://example.com/jobs/three",
          "descriptionHtml": null,
          "employmentType": null,
          "id": "3",
          "locations": [{ "name": "Remote" }],
          "publishedAt": null,
          "sourceId": "source",
          "title": "Third",
          "workplace": null,
        },
        {
          "applyUrl": "https://example.com/jobs/two",
          "descriptionHtml": null,
          "employmentType": null,
          "id": "2",
          "locations": [{ "name": "Remote" }],
          "publishedAt": null,
          "sourceId": "source",
          "title": "Second",
          "workplace": null,
        },
      ]),
    );

    let response = test
      .get(&format!(
        "/v1/jobs?limit=2&cursor={}",
        first_page["nextCursor"].as_str().unwrap()
      ))
      .await;

    assert_eq!(response.status(), StatusCode::OK);

    let second_page = response_json(response).await;

    assert_eq!(
      second_page,
      json!({
        "jobs": [{
          "applyUrl": "https://example.com/jobs/one",
          "descriptionHtml": null,
          "employmentType": null,
          "id": "1",
          "locations": [{ "name": "Remote" }],
          "publishedAt": null,
          "sourceId": "source",
          "title": "First",
          "workplace": null,
        }],
        "nextCursor": null,
      }),
    );
  }

  #[tokio::test]
  async fn jobs_route_rejects_invalid_pagination() {
    let test = Test::new().await;

    let response = test.get("/v1/jobs?cursor=not-a-cursor").await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    assert_eq!(
      response_json(response).await,
      json!({ "error": "invalid cursor" }),
    );

    let response = test.get("/v1/jobs?limit=0").await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    assert_eq!(
      response_json(response).await,
      json!({ "error": "limit must be between 1 and 100" }),
    );

    let response = test.get("/v1/jobs?limit=101").await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    assert_eq!(
      response_json(response).await,
      json!({ "error": "limit must be between 1 and 100" }),
    );
  }

  #[tokio::test]
  async fn jobs_route_returns_only_open_jobs_without_private_data() {
    let test = Test::new().await;

    let first = JobDraft {
      apply_url: "https://example.com/jobs/one".parse().unwrap(),
      description_html: Some(
        "<p>First</p><script>alert('unsafe')</script>".into(),
      ),
      employment_type: None,
      external_id: "one".into(),
      locations: vec![JobLocation {
        name: "Remote".into(),
      }],
      published_at: None,
      raw: json!({ "private": true }),
      title: "First".into(),
      workplace: None,
    };

    let second = JobDraft {
      apply_url: "https://example.com/jobs/two".parse().unwrap(),
      description_html: None,
      external_id: "two".into(),
      title: "Second".into(),
      ..first.clone()
    };

    test.sync(vec![first.clone(), second]).await;
    test.sync(vec![first]).await;

    let response = test.get("/v1/jobs").await;

    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;

    assert_eq!(
      body,
      json!({
        "jobs": [{
          "applyUrl": "https://example.com/jobs/one",
          "descriptionHtml": "<p>First</p>",
          "employmentType": null,
          "id": "1",
          "locations": [{ "name": "Remote" }],
          "publishedAt": null,
          "sourceId": "source",
          "title": "First",
          "workplace": null,
        }],
        "nextCursor": null,
      }),
    );
  }
}
