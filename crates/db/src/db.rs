use super::*;

#[derive(Clone, Debug)]
pub struct Db {
  pool: PgPool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SyncSummary {
  pub jobs_closed: i32,
  pub jobs_seen: i32,
  pub jobs_upserted: i32,
}

impl Db {
  pub async fn close(&self) {
    self.pool.close().await;
  }

  /// Applies a complete job snapshot and marks its sync run as successful.
  ///
  /// All job changes and sync-run counts are committed atomically. Jobs that
  /// were not seen in this snapshot are closed, and previously closed jobs
  /// that reappear are reopened.
  ///
  /// # Errors
  ///
  /// Returns an error if the sync run is not running, was superseded by a
  /// newer successful run, a count exceeds the database range, or a query
  /// fails.
  pub async fn complete_sync(
    &self,
    sync_run_id: i64,
    source_id: &str,
    snapshot: &JobSnapshot,
  ) -> Result<SyncSummary> {
    let jobs_seen =
      i32::try_from(snapshot.jobs.len()).map_err(Error::CountOverflow)?;

    let mut transaction = self.pool.begin().await?;

    let source = sqlx::query_scalar::<_, String>(
      "SELECT id FROM sources WHERE id = $1 FOR UPDATE",
    )
    .bind(source_id)
    .fetch_optional(&mut *transaction)
    .await?;

    if source.is_none() {
      return Err(Error::SyncRunNotRunning(sync_run_id));
    }

    let running = sqlx::query_scalar::<_, i64>(
      "SELECT id
       FROM sync_runs
       WHERE id = $1 AND source_id = $2 AND status = 'running'
       FOR UPDATE",
    )
    .bind(sync_run_id)
    .bind(source_id)
    .fetch_optional(&mut *transaction)
    .await?;

    if running.is_none() {
      return Err(Error::SyncRunNotRunning(sync_run_id));
    }

    let superseded = sqlx::query_scalar::<_, bool>(
      "SELECT EXISTS (
         SELECT 1
         FROM sync_runs
         WHERE source_id = $1 AND id > $2 AND status = 'succeeded'
       )",
    )
    .bind(source_id)
    .bind(sync_run_id)
    .fetch_one(&mut *transaction)
    .await?;

    if superseded {
      return Err(Error::SyncRunSuperseded(sync_run_id));
    }

    let mut jobs_upserted = 0_u64;

    for job in &snapshot.jobs {
      jobs_upserted +=
        Self::upsert_job(&mut transaction, sync_run_id, source_id, job).await?;
    }

    let jobs_closed = sqlx::query(
      "UPDATE jobs
       SET closed_at = NOW()
       WHERE source_id = $1
         AND closed_at IS NULL
         AND last_seen_sync_run_id IS DISTINCT FROM $2",
    )
    .bind(source_id)
    .bind(sync_run_id)
    .execute(&mut *transaction)
    .await?
    .rows_affected();

    let jobs_closed =
      i32::try_from(jobs_closed).map_err(Error::CountOverflow)?;
    let jobs_upserted =
      i32::try_from(jobs_upserted).map_err(Error::CountOverflow)?;

    let result = sqlx::query(
      "UPDATE sync_runs
       SET status = 'succeeded',
           jobs_seen = $2,
           jobs_upserted = $3,
           jobs_closed = $4,
           error = NULL,
           finished_at = NOW()
       WHERE id = $1 AND source_id = $5 AND status = 'running'",
    )
    .bind(sync_run_id)
    .bind(jobs_seen)
    .bind(jobs_upserted)
    .bind(jobs_closed)
    .bind(source_id)
    .execute(&mut *transaction)
    .await?;

    if result.rows_affected() != 1 {
      return Err(Error::SyncRunNotRunning(sync_run_id));
    }

    transaction.commit().await?;

    Ok(SyncSummary {
      jobs_closed,
      jobs_seen,
      jobs_upserted,
    })
  }

  /// Connects to `PostgreSQL` and runs all pending migrations.
  ///
  /// # Errors
  ///
  /// Returns an error if `PostgreSQL` cannot be reached or a migration fails.
  pub async fn connect(url: &str) -> Result<Self> {
    let pool = PgPoolOptions::new()
      .max_connections(10)
      .connect(url)
      .await
      .map_err(Error::Connect)?;

    sqlx::migrate!()
      .run(&pool)
      .await
      .map_err(Error::Migration)?;

    Ok(Self { pool })
  }

  /// Marks a running sync as failed without changing any jobs.
  ///
  /// # Errors
  ///
  /// Returns an error if the sync run is not running or the query fails.
  pub async fn fail_sync(&self, sync_run_id: i64, error: &str) -> Result {
    let result = sqlx::query(
      "UPDATE sync_runs
       SET status = 'failed', error = $2, finished_at = NOW()
       WHERE id = $1 AND status = 'running'",
    )
    .bind(sync_run_id)
    .bind(error)
    .execute(&self.pool)
    .await?;

    if result.rows_affected() != 1 {
      return Err(Error::SyncRunNotRunning(sync_run_id));
    }

    Ok(())
  }

  /// Inserts a job.
  ///
  /// # Errors
  ///
  /// Returns an error if the query fails.
  pub async fn insert_job(&self, job: &Job) -> Result {
    sqlx::query(
      "INSERT INTO jobs (
         source_id,
         external_id,
         title,
         description_html,
         apply_url,
         locations,
         workplace,
         employment_type,
         published_at,
         raw
       ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(&job.source_id)
    .bind(&job.external_id)
    .bind(&job.title)
    .bind(job.description_html.as_deref())
    .bind(&job.apply_url)
    .bind(Json(&job.locations))
    .bind(job.workplace.as_deref())
    .bind(job.employment_type.as_deref())
    .bind(job.published_at.as_ref())
    .bind(Json(&job.raw))
    .execute(&self.pool)
    .await?;

    Ok(())
  }

  /// Inserts a source.
  ///
  /// # Errors
  ///
  /// Returns an error if the query fails.
  pub async fn insert_source(&self, source: &Source) -> Result {
    sqlx::query(
      "INSERT INTO sources (
         id, organization, adapter, configuration, enabled
       ) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(&source.id)
    .bind(&source.organization)
    .bind(&source.adapter)
    .bind(Json(&source.configuration))
    .bind(source.enabled)
    .execute(&self.pool)
    .await?;

    Ok(())
  }

  /// Verifies that `PostgreSQL` is reachable.
  ///
  /// # Errors
  ///
  /// Returns an error if the probe query fails.
  pub async fn ping(&self) -> Result {
    sqlx::query("SELECT 1").execute(&self.pool).await?;

    Ok(())
  }

  /// Upserts a source and records the start of a sync run.
  ///
  /// # Errors
  ///
  /// Returns an error if a query fails.
  pub async fn start_sync(&self, source: &Source) -> Result<i64> {
    let mut transaction = self.pool.begin().await?;

    sqlx::query(
      "INSERT INTO sources (
         id, organization, adapter, configuration, enabled
       ) VALUES ($1, $2, $3, $4, $5)
       ON CONFLICT (id) DO UPDATE SET
         organization = EXCLUDED.organization,
         adapter = EXCLUDED.adapter,
         configuration = EXCLUDED.configuration,
         enabled = EXCLUDED.enabled",
    )
    .bind(&source.id)
    .bind(&source.organization)
    .bind(&source.adapter)
    .bind(Json(&source.configuration))
    .bind(source.enabled)
    .execute(&mut *transaction)
    .await?;

    let sync_run_id = sqlx::query_scalar::<_, i64>(
      "INSERT INTO sync_runs (source_id, status)
       VALUES ($1, 'running')
       RETURNING id",
    )
    .bind(&source.id)
    .fetch_one(&mut *transaction)
    .await?;

    transaction.commit().await?;

    Ok(sync_run_id)
  }

  async fn upsert_job(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    sync_run_id: i64,
    source_id: &str,
    job: &JobDraft,
  ) -> Result<u64> {
    let result = sqlx::query(
      "INSERT INTO jobs (
         source_id,
         external_id,
         title,
         description_html,
         apply_url,
         locations,
         workplace,
         employment_type,
         published_at,
         raw,
         last_seen_sync_run_id
       ) VALUES (
         $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11
       )
       ON CONFLICT (source_id, external_id) DO UPDATE SET
         title = EXCLUDED.title,
         description_html = EXCLUDED.description_html,
         apply_url = EXCLUDED.apply_url,
         locations = EXCLUDED.locations,
         workplace = EXCLUDED.workplace,
         employment_type = EXCLUDED.employment_type,
         published_at = EXCLUDED.published_at,
         last_seen_at = NOW(),
         closed_at = NULL,
         last_seen_sync_run_id = EXCLUDED.last_seen_sync_run_id,
         raw = EXCLUDED.raw",
    )
    .bind(source_id)
    .bind(&job.external_id)
    .bind(&job.title)
    .bind(job.description_html.as_deref())
    .bind(job.apply_url.as_str())
    .bind(Json(&job.locations))
    .bind(job.workplace.map(Workplace::as_str))
    .bind(job.employment_type.map(EmploymentType::as_str))
    .bind(job.published_at.as_ref())
    .bind(Json(&job.raw))
    .bind(sync_run_id)
    .execute(&mut **transaction)
    .await?;

    Ok(result.rows_affected())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  struct Test {
    db: Db,
  }

  impl Test {
    async fn new() -> Self {
      let number = TEST_DATABASE_NUMBER.fetch_add(1, Ordering::Relaxed);

      let name = format!(
        "jobs-surf-test-{}-{}-{}",
        process::id(),
        SystemTime::now()
          .duration_since(UNIX_EPOCH)
          .unwrap()
          .as_millis(),
        number,
      );

      let url = format!("postgres://jobs_surf:jobs_surf@localhost:5432/{name}");

      Postgres::create_database(&url).await.unwrap();

      Self {
        db: Db::connect(&url).await.unwrap(),
      }
    }
  }

  #[allow(clippy::too_many_lines)]
  #[tokio::test]
  async fn complete_snapshots_close_and_reopen_jobs() {
    let Test { db } = Test::new().await;

    let source = Source {
      enabled: true,
      id: "source".into(),
      ..Default::default()
    };

    let first_job = JobDraft {
      apply_url: "https://example.com/jobs/one".parse().unwrap(),
      description_html: Some("<p>First</p>".into()),
      employment_type: None,
      external_id: "one".into(),
      locations: vec![JobLocation {
        name: "Remote".into(),
      }],
      published_at: None,
      raw: JsonValue::default(),
      title: "First".into(),
      workplace: None,
    };

    let second_job = JobDraft {
      apply_url: "https://example.com/jobs/two".parse().unwrap(),
      description_html: Some("<p>Second</p>".into()),
      employment_type: None,
      external_id: "two".into(),
      locations: vec![JobLocation {
        name: "Remote".into(),
      }],
      published_at: None,
      raw: JsonValue::default(),
      title: "Second".into(),
      workplace: None,
    };

    let first_run = db.start_sync(&source).await.unwrap();
    let summary = db
      .complete_sync(
        first_run,
        &source.id,
        &JobSnapshot {
          jobs: vec![first_job.clone(), second_job.clone()],
        },
      )
      .await
      .unwrap();

    assert_eq!(
      summary,
      SyncSummary {
        jobs_closed: 0,
        jobs_seen: 2,
        jobs_upserted: 2,
      },
    );

    let updated_job = JobDraft {
      description_html: Some("<p>Updated</p>".into()),
      title: "Updated".into(),
      ..first_job
    };

    let second_run = db.start_sync(&source).await.unwrap();
    let summary = db
      .complete_sync(
        second_run,
        &source.id,
        &JobSnapshot {
          jobs: vec![updated_job.clone()],
        },
      )
      .await
      .unwrap();

    assert_eq!(
      summary,
      SyncSummary {
        jobs_closed: 1,
        jobs_seen: 1,
        jobs_upserted: 1,
      },
    );

    let jobs = sqlx::query_as::<_, (String, bool, String)>(
      "SELECT external_id, closed_at IS NOT NULL, title
       FROM jobs
       WHERE source_id = $1
       ORDER BY external_id",
    )
    .bind(&source.id)
    .fetch_all(&db.pool)
    .await
    .unwrap();

    assert_eq!(
      jobs,
      vec![
        ("one".into(), false, "Updated".into()),
        ("two".into(), true, "Second".into()),
      ],
    );

    let returned_job = JobDraft {
      description_html: Some("<p>Returned</p>".into()),
      title: "Returned".into(),
      ..second_job
    };

    let third_run = db.start_sync(&source).await.unwrap();
    let summary = db
      .complete_sync(
        third_run,
        &source.id,
        &JobSnapshot {
          jobs: vec![updated_job, returned_job],
        },
      )
      .await
      .unwrap();

    assert_eq!(
      summary,
      SyncSummary {
        jobs_closed: 0,
        jobs_seen: 2,
        jobs_upserted: 2,
      },
    );

    let open_jobs = sqlx::query_scalar::<_, i64>(
      "SELECT COUNT(*) FROM jobs
       WHERE source_id = $1 AND closed_at IS NULL",
    )
    .bind(&source.id)
    .fetch_one(&db.pool)
    .await
    .unwrap();

    assert_eq!(open_jobs, 2);
  }

  #[tokio::test]
  async fn failed_sync_does_not_change_jobs() {
    let Test { db } = Test::new().await;

    let source = Source {
      enabled: true,
      id: "source".into(),
      ..Default::default()
    };

    let first_run = db.start_sync(&source).await.unwrap();

    let job = JobDraft {
      apply_url: "https://example.com/jobs/one".parse().unwrap(),
      description_html: Some("<p>First</p>".into()),
      employment_type: None,
      external_id: "one".into(),
      locations: vec![JobLocation {
        name: "Remote".into(),
      }],
      published_at: None,
      raw: JsonValue::default(),
      title: "First".into(),
      workplace: None,
    };

    db.complete_sync(first_run, &source.id, &JobSnapshot { jobs: vec![job] })
      .await
      .unwrap();

    let failed_run = db.start_sync(&source).await.unwrap();
    db.fail_sync(failed_run, "provider unavailable")
      .await
      .unwrap();

    let job = sqlx::query_as::<_, (bool, i64)>(
      "SELECT closed_at IS NULL, last_seen_sync_run_id
       FROM jobs
       WHERE source_id = $1 AND external_id = 'one'",
    )
    .bind(&source.id)
    .fetch_one(&db.pool)
    .await
    .unwrap();

    assert_eq!(job, (true, first_run));

    let run = sqlx::query_as::<_, (String, String)>(
      "SELECT status, error FROM sync_runs WHERE id = $1",
    )
    .bind(failed_run)
    .fetch_one(&db.pool)
    .await
    .unwrap();

    assert_eq!(run, ("failed".into(), "provider unavailable".into()));
  }

  #[tokio::test]
  async fn jobs_are_unique_within_source() {
    let Test { db } = Test::new().await;

    let source = Source {
      id: "source".into(),
      ..Default::default()
    };

    let job = Job {
      source_id: source.id.clone(),
      external_id: "external-id".into(),
      ..Default::default()
    };

    db.insert_source(&source).await.unwrap();
    db.insert_job(&job).await.unwrap();

    let Error::Query(error) = db.insert_job(&job).await.unwrap_err() else {
      panic!("expected query error");
    };

    assert_eq!(
      error
        .as_database_error()
        .and_then(|error| error.constraint()),
      Some("jobs_source_external_id_unique"),
    );
  }

  #[tokio::test]
  async fn newer_successful_sync_supersedes_an_older_run() {
    let Test { db } = Test::new().await;

    let source = Source {
      enabled: true,
      id: "source".into(),
      ..Default::default()
    };

    let older_run = db.start_sync(&source).await.unwrap();
    let newer_run = db.start_sync(&source).await.unwrap();

    let job = JobDraft {
      apply_url: "https://example.com/jobs/one".parse().unwrap(),
      description_html: None,
      employment_type: None,
      external_id: "one".into(),
      locations: Vec::new(),
      published_at: None,
      raw: JsonValue::default(),
      title: "Newer snapshot".into(),
      workplace: None,
    };

    db.complete_sync(newer_run, &source.id, &JobSnapshot { jobs: vec![job] })
      .await
      .unwrap();

    assert!(matches!(
      db
        .complete_sync(
          older_run,
          &source.id,
          &JobSnapshot { jobs: Vec::new() },
        )
        .await,
      Err(Error::SyncRunSuperseded(id)) if id == older_run,
    ));

    db.fail_sync(older_run, "superseded").await.unwrap();

    let job = sqlx::query_as::<_, (bool, String, i64)>(
      "SELECT closed_at IS NULL, title, last_seen_sync_run_id
       FROM jobs
       WHERE source_id = $1 AND external_id = 'one'",
    )
    .bind(&source.id)
    .fetch_one(&db.pool)
    .await
    .unwrap();

    assert_eq!(job, (true, "Newer snapshot".into(), newer_run));
  }
}
