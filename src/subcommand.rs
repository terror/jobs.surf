use super::*;

mod serve;
mod sync;

#[derive(Debug, Parser)]
pub(crate) enum Subcommand {
  #[command(about = "Start the server")]
  Serve(serve::Serve),
  #[command(about = "Synchronize a configured job source")]
  Sync(sync::Sync),
}

impl Subcommand {
  pub(crate) async fn run(self, options: Options) -> Result {
    match self {
      Self::Serve(serve) => serve.run(options).await,
      Self::Sync(sync) => sync.run(options).await,
    }
  }
}
