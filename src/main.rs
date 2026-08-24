use {
  crate::{
    arguments::Arguments, options::Options, state::State,
    subcommand::Subcommand,
  },
  ammonia::clean,
  anyhow::Context,
  axum::{
    Json, Router,
    extract::{Query, State as AppState},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
  },
  base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD},
  chrono::{DateTime, Utc},
  clap::{Args, Parser},
  dotenv::dotenv,
  jobs_surf::config::Config,
  jobs_surf_db::{Db, JobCursor, JobRecord},
  jobs_surf_model::{JobLocation, Source},
  serde::{Deserialize, Serialize},
  std::{fs, net::SocketAddr, num::NonZeroU16, path::PathBuf, process},
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
  jobs_surf_model::{JobDraft, JobSnapshot},
  serde_json::{Value, json},
  sqlx::{Postgres, migrate::MigrateDatabase},
  std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::{SystemTime, UNIX_EPOCH},
  },
  tower::ServiceExt,
};

mod arguments;
mod health;
mod jobs;
mod options;
mod state;
mod subcommand;

type Result<T = (), E = anyhow::Error> = std::result::Result<T, E>;

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
