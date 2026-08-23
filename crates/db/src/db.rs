use super::*;

#[derive(Clone, Debug)]
pub struct Db {
  pool: PgPool,
}

impl Db {
  pub async fn close(&self) {
    self.pool.close().await;
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
}
