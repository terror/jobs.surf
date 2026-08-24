use super::*;

#[derive(Args, Debug)]
pub(crate) struct Sync {
  #[arg(
    default_value = "config.toml",
    env = "JOBS_SURF_CONFIG",
    help = "Path to the source configuration file",
    long
  )]
  config: PathBuf,
  #[arg(help = "ID of the source to synchronize", long)]
  source: String,
}

impl Sync {
  pub(crate) async fn run(self, options: Options) -> Result {
    let input = fs::read_to_string(&self.config).with_context(|| {
      format!("failed to read config `{}`", self.config.display())
    })?;

    let config = Config::from_toml(&input).with_context(|| {
      format!("failed to parse config `{}`", self.config.display())
    })?;

    let mut sources = config
      .sources
      .into_iter()
      .filter(|source| source.id == self.source);

    let source = sources
      .next()
      .with_context(|| format!("source `{}` is not configured", self.source))?;

    if sources.next().is_some() {
      anyhow::bail!("source `{}` is configured more than once", self.source);
    }

    if !source.enabled {
      anyhow::bail!("source `{}` is disabled", source.id);
    }

    let (adapter, configuration) = source.adapter.adapter();

    let database_source = Source {
      adapter: source.adapter.kind().into(),
      configuration,
      enabled: source.enabled,
      id: source.id,
      organization: source.organization,
    };

    let db = Db::connect(&options.db_url).await?;

    synchronize(&db, &database_source, adapter.as_ref()).await
  }
}

pub(super) async fn synchronize(
  db: &Db,
  source: &Source,
  adapter: &dyn Adapter,
) -> Result {
  let sync_run_id = db.start_sync(source).await?;

  let outcome: Result<_> = async {
    let snapshot = adapter.fetch().await?;

    snapshot.validate()?;

    db.complete_sync(sync_run_id, &source.id, &snapshot)
      .await
      .map_err(Into::into)
  }
  .await;

  match outcome {
    Ok(summary) => {
      info!(
        source = %source.id,
        sync_run_id,
        jobs_seen = summary.jobs_seen,
        jobs_upserted = summary.jobs_upserted,
        jobs_closed = summary.jobs_closed,
        "sync completed",
      );

      Ok(())
    }
    Err(sync_error) => {
      let error_message = format!("{sync_error:#}");

      if let Err(mark_error) = db.fail_sync(sync_run_id, &error_message).await {
        return Err(anyhow::anyhow!(
          "{sync_error:#}; additionally failed to mark sync run \
           `{sync_run_id}` as failed: {mark_error}"
        ));
      }

      Err(sync_error)
    }
  }
}
