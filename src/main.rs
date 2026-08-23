use {
  crate::{
    arguments::Arguments, options::Options, state::State,
    subcommand::Subcommand,
  },
  anyhow::Context,
  axum::{Router, extract::State as AppState, http::StatusCode, routing::get},
  clap::{Args, Parser},
  dotenv::dotenv,
  jobs_surf_db::Db,
  std::{net::SocketAddr, process},
  tokio::net::TcpListener,
  tower_http::trace::TraceLayer,
  tracing::{error, info},
  tracing_subscriber::EnvFilter,
};

#[cfg(test)]
use {
  axum::{
    body::{Body, to_bytes},
    http::{Method, Request},
  },
  sqlx::{Postgres, migrate::MigrateDatabase},
  std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::{SystemTime, UNIX_EPOCH},
  },
  tower::ServiceExt,
};

mod arguments;
mod health;
mod options;
mod state;
mod subcommand;

type Result<T = (), E = anyhow::Error> = std::result::Result<T, E>;

#[cfg(test)]
static TEST_DATABASE_NUMBER: AtomicUsize = AtomicUsize::new(0);

#[tokio::main]
async fn main() {
  dotenv().ok();

  tracing_subscriber::fmt()
    .with_env_filter(
      EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,tower_http=debug".into()),
    )
    .init();

  if let Err(error) = Arguments::parse().run().await {
    eprintln!("error: {error:#}");
    process::exit(1);
  }
}
