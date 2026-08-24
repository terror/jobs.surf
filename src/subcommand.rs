use super::*;

mod openapi;
mod serve;
mod sync;

#[derive(Debug, Parser)]
pub(crate) enum Subcommand {
  #[command(about = "Write the OpenAPI document")]
  Openapi(openapi::Openapi),
  #[command(about = "Start the server")]
  Serve(serve::Serve),
  #[command(about = "Synchronize configured job sources")]
  Sync(sync::Sync),
}

impl Subcommand {
  pub(crate) async fn run(self, options: Options) -> Result {
    match self {
      Self::Openapi(openapi) => openapi.run(),
      Self::Serve(serve) => serve.run(options).await,
      Self::Sync(sync) => sync.run(options).await,
    }
  }
}
