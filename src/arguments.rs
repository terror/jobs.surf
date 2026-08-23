use super::*;

#[derive(Debug, Parser)]
#[command(version, about)]
pub(crate) struct Arguments {
  #[command(flatten)]
  options: Options,
  #[command(subcommand)]
  subcommand: Subcommand,
}

impl Arguments {
  pub(crate) async fn run(self) -> Result {
    self.subcommand.run(self.options).await
  }
}
