use super::*;

#[derive(Args, Debug)]
pub(crate) struct Serve {
  #[arg(long, default_value = "3000", help = "Port to listen on")]
  port: u16,
}

impl Serve {
  fn app(db: Db) -> Router {
    Router::new()
      .merge(Scalar::with_url("/docs", Documentation::openapi()))
      .route("/healthz", get(health::get_health))
      .route("/v1/jobs", get(jobs::get_jobs))
      .route("/v1/jobs/{id}", get(jobs::get_job))
      .route("/v1/sources", get(sources::get_sources))
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

      self.sync_source(&source, jobs).await;
    }

    async fn sync_source(&self, source: &Source, jobs: Vec<JobDraft>) {
      let sync_run_id = self.db.start_sync(source).await.unwrap();

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
  async fn job_route_returns_only_an_open_job() {
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
      workplace: Some(Workplace::Remote),
    };

    let second = JobDraft {
      apply_url: "https://example.com/jobs/two".parse().unwrap(),
      external_id: "two".into(),
      title: "Second".into(),
      ..first.clone()
    };

    test.sync(vec![first.clone(), second.clone()]).await;

    let response = test.get("/v1/jobs/1").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
      response_json(response).await,
      json!({
        "applyUrl": "https://example.com/jobs/one",
        "descriptionHtml": "<p>First</p>",
        "employmentType": null,
        "id": "1",
        "locations": [{ "name": "Remote" }],
        "publishedAt": null,
        "sourceId": "source",
        "title": "First",
        "workplace": "remote",
      }),
    );

    test.sync(vec![second]).await;

    let response = test.get("/v1/jobs/1").await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
      response_json(response).await,
      json!({ "error": "job not found" }),
    );

    let response = test.get("/v1/jobs/not-a-number").await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
      response_json(response).await,
      json!({ "error": "invalid job id" }),
    );

    let response = test.get("/v1/jobs/999").await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
  }

  #[tokio::test]
  async fn jobs_route_applies_filters_and_search() {
    let test = Test::new().await;

    let first_source = Source {
      adapter: "greenhouse".into(),
      enabled: true,
      id: "first-source".into(),
      organization: "First".into(),
      ..Default::default()
    };

    let second_source = Source {
      adapter: "greenhouse".into(),
      enabled: true,
      id: "second-source".into(),
      organization: "Second".into(),
      ..Default::default()
    };

    let remote = JobDraft {
      apply_url: "https://example.com/jobs/remote".parse().unwrap(),
      description_html: Some("<p>Distributed systems</p>".into()),
      employment_type: None,
      external_id: "remote".into(),
      locations: vec![JobLocation {
        name: "Anywhere".into(),
      }],
      published_at: None,
      raw: Value::Null,
      title: "Rust platform engineer".into(),
      workplace: Some(Workplace::Remote),
    };

    let hybrid = JobDraft {
      apply_url: "https://example.com/jobs/hybrid".parse().unwrap(),
      description_html: Some("<p>TypeScript interfaces</p>".into()),
      external_id: "hybrid".into(),
      title: "Frontend engineer".into(),
      workplace: Some(Workplace::Hybrid),
      ..remote.clone()
    };

    let unclassified = JobDraft {
      apply_url: "https://example.com/jobs/database".parse().unwrap(),
      description_html: Some("<p>PostgreSQL and Rust</p>".into()),
      external_id: "database".into(),
      title: "Database engineer".into(),
      workplace: None,
      ..remote.clone()
    };

    test
      .sync_source(&first_source, vec![remote.clone(), hybrid.clone()])
      .await;
    test.sync_source(&second_source, vec![unclassified]).await;

    let response = test.get("/v1/jobs?source=first-source").await;
    let body = response_json(response).await;
    assert_eq!(body["jobs"].as_array().unwrap().len(), 2);
    assert!(
      body["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .all(|job| job["sourceId"] == "first-source"),
    );

    let response = test.get("/v1/jobs?remote=true").await;
    let body = response_json(response).await;
    assert_eq!(body["jobs"].as_array().unwrap().len(), 1);
    assert_eq!(body["jobs"][0]["title"], "Rust platform engineer");

    let response = test.get("/v1/jobs?remote=false").await;
    let body = response_json(response).await;
    assert_eq!(body["jobs"].as_array().unwrap().len(), 2);

    let response = test.get("/v1/jobs?query=rust").await;
    let body = response_json(response).await;
    assert_eq!(body["jobs"].as_array().unwrap().len(), 2);

    let response = test
      .get("/v1/jobs?query=rust&remote=true&source=first-source")
      .await;
    let body = response_json(response).await;
    assert_eq!(body["jobs"].as_array().unwrap().len(), 1);
    assert_eq!(body["jobs"][0]["title"], "Rust platform engineer");

    let updated = JobDraft {
      description_html: Some("<p>Systems programming</p>".into()),
      title: "Zig platform engineer".into(),
      ..remote
    };

    test.sync_source(&first_source, vec![updated, hybrid]).await;

    let response = test.get("/v1/jobs?query=rust&source=first-source").await;
    let body = response_json(response).await;
    assert!(body["jobs"].as_array().unwrap().is_empty());
  }

  #[tokio::test]
  async fn jobs_route_filters_before_paginating() {
    let test = Test::new().await;

    let template = JobDraft {
      apply_url: "https://example.com/jobs/one".parse().unwrap(),
      description_html: None,
      employment_type: None,
      external_id: "one".into(),
      locations: Vec::new(),
      published_at: None,
      raw: Value::Null,
      title: "Rust one".into(),
      workplace: Some(Workplace::Remote),
    };

    let jobs = ["Rust one", "Go two", "Rust three", "Go four", "Rust five"]
      .into_iter()
      .enumerate()
      .map(|(index, title)| JobDraft {
        apply_url: format!("https://example.com/jobs/{index}").parse().unwrap(),
        external_id: index.to_string(),
        title: title.into(),
        ..template.clone()
      })
      .collect();

    test.sync(jobs).await;

    let response = test.get("/v1/jobs?limit=2&query=rust").await;
    assert_eq!(response.status(), StatusCode::OK);
    let first_page = response_json(response).await;

    assert_eq!(first_page["jobs"].as_array().unwrap().len(), 2);

    let response = test
      .get(&format!(
        "/v1/jobs?limit=2&query=rust&cursor={}",
        first_page["nextCursor"].as_str().unwrap(),
      ))
      .await;
    let second_page = response_json(response).await;

    assert_eq!(second_page["jobs"].as_array().unwrap().len(), 1);
    assert_eq!(second_page["nextCursor"], Value::Null);

    let first_ids = first_page["jobs"]
      .as_array()
      .unwrap()
      .iter()
      .map(|job| job["id"].as_str().unwrap())
      .collect::<Vec<_>>();
    let second_id = second_page["jobs"][0]["id"].as_str().unwrap();

    assert!(!first_ids.contains(&second_id));
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

    let response = test.get("/v1/jobs?remote=perhaps").await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
      response_json(response).await,
      json!({ "error": "invalid query parameters" }),
    );

    let response = test.get("/v1/jobs?unknown=value").await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
      response_json(response).await,
      json!({ "error": "invalid query parameters" }),
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

  #[tokio::test]
  async fn sources_route_returns_only_public_fields_in_display_order() {
    let test = Test::new().await;

    let response = test.get("/v1/sources").await;
    assert_eq!(response_json(response).await, json!({ "sources": [] }),);

    for source in [
      Source {
        adapter: "greenhouse".into(),
        configuration: json!({ "board_token": "private-token" }),
        enabled: true,
        id: "zeta".into(),
        organization: "Acme".into(),
      },
      Source {
        adapter: "ashby".into(),
        enabled: true,
        id: "beta".into(),
        organization: "Beta".into(),
        ..Default::default()
      },
      Source {
        adapter: "greenhouse".into(),
        enabled: true,
        id: "alpha".into(),
        organization: "Acme".into(),
        ..Default::default()
      },
    ] {
      test.sync_source(&source, Vec::new()).await;
    }

    let response = test.get("/v1/sources").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
      response_json(response).await,
      json!({
        "sources": [
          {
            "adapter": "greenhouse",
            "id": "alpha",
            "organization": "Acme",
          },
          {
            "adapter": "greenhouse",
            "id": "zeta",
            "organization": "Acme",
          },
          {
            "adapter": "ashby",
            "id": "beta",
            "organization": "Beta",
          },
        ],
      }),
    );
  }
}
