use super::*;

#[derive(Args, Debug)]
pub(crate) struct Server {
  #[arg(long, default_value = "8000", help = "Port to listen on")]
  port: u16,
}

impl Server {
  pub(crate) async fn run(self, options: Options) -> Result {
    let db = PgPoolOptions::new()
      .max_connections(10)
      .connect(&options.database_url)
      .await
      .context("failed to connect to postgres")?;

    let addr = SocketAddr::from(([0, 0, 0, 0], self.port));

    let listener = TcpListener::bind(addr)
      .await
      .with_context(|| format!("failed to bind to {addr}"))?;

    info!(%addr, "listening");

    axum::serve(listener, Self::app(db)).await?;

    Ok(())
  }

  fn app(db: PgPool) -> Router {
    Router::new()
      .route("/api/health", get(health::get_health))
      .with_state(State { db })
      .layer(TraceLayer::new_for_http())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  struct Test {
    app: Router,
    db: PgPool,
  }

  impl Test {
    async fn new() -> Self {
      dotenv().ok();

      let database_url = env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
        "postgres://jobs_surf:jobs_surf@localhost:5432/jobs_surf".into()
      });

      let db = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .unwrap();

      Self {
        app: Server::app(db.clone()),
        db,
      }
    }
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
}
