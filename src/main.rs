use {
  crate::{
    arguments::Arguments, options::Options, state::State,
    subcommand::Subcommand,
  },
  anyhow::Context,
  axum::Router,
  clap::{Args, Parser},
  dotenv::dotenv,
  jobs_surf_db::Db,
  std::{net::SocketAddr, process},
  tokio::net::TcpListener,
  tower_http::trace::TraceLayer,
  tracing::info,
  tracing_subscriber::EnvFilter,
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
