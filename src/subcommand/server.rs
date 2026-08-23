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
