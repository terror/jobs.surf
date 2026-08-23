use super::*;

mod server;

#[derive(Debug, Parser)]
pub(crate) enum Subcommand {
  #[command(about = "Start the server")]
  Server(server::Server),
}

impl Subcommand {
  pub(crate) async fn run(self, options: Options) -> Result {
    match self {
      Self::Server(server) => server.run(options).await,
    }
  }
}
