use super::*;

#[derive(Args, Debug)]
pub(crate) struct Openapi {
  #[arg(help = "Path to write the OpenAPI document", long)]
  output: PathBuf,
}

impl Openapi {
  pub(crate) fn run(self) -> Result {
    if let Some(parent) = self.output.parent() {
      fs::create_dir_all(parent).with_context(|| {
        format!("failed to create directory `{}`", parent.display())
      })?;
    }

    let mut document = serde_json::to_string_pretty(&Documentation::openapi())?;

    document.push('\n');

    fs::write(&self.output, document).with_context(|| {
      format!(
        "failed to write OpenAPI document `{}`",
        self.output.display(),
      )
    })?;

    Ok(())
  }
}
