use super::*;

mod server;
mod sync;

#[derive(Debug, Parser)]
pub(crate) enum Subcommand {
  #[command(about = "Start the server")]
  Server(server::Server),
  #[command(about = "Synchronize a configured job source")]
  Sync(sync::Sync),
}

impl Subcommand {
  pub(crate) async fn run(self, options: Options) -> Result {
    match self {
      Self::Server(server) => server.run(options).await,
      Self::Sync(sync) => sync.run(options).await,
    }
  }
}
