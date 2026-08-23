use super::*;

#[derive(Args, Debug)]
pub(crate) struct Server {
  #[arg(long, default_value = "8000", help = "Port to listen on")]
  port: u16,
}

impl Server {
  fn app(db: Db) -> Router {
    Router::new()
      .route("/api/health", get(health::get_health))
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

  struct Test {
    app: Router,
    db: Db,
  }

  impl Test {
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
        app: Server::app(db.clone()),
        db,
      }
    }
  }

  #[tokio::test]
  async fn health_route_reports_database_failure() {
    let Test { app, db } = Test::new().await;

    db.close().await;

    let response = app
      .oneshot(
        Request::builder()
          .method(Method::GET)
          .uri("/api/health")
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
  }

  #[tokio::test]
  async fn health_route_works() {
    let Test { app, .. } = Test::new().await;

    let response = app
      .oneshot(
        Request::builder()
          .method(Method::GET)
          .uri("/api/health")
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
      to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
        .as_ref(),
      b"ok",
    );
  }
}
