CREATE TABLE sources (
  id TEXT PRIMARY KEY,
  organization TEXT NOT NULL,
  adapter TEXT NOT NULL,
  configuration JSONB NOT NULL DEFAULT '{}'::JSONB,
  enabled BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE sync_runs (
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  source_id TEXT NOT NULL REFERENCES sources(id),
  status TEXT NOT NULL CHECK (status IN ('running', 'succeeded', 'failed')),
  jobs_seen INTEGER NOT NULL DEFAULT 0 CHECK (jobs_seen >= 0),
  jobs_upserted INTEGER NOT NULL DEFAULT 0 CHECK (jobs_upserted >= 0),
  jobs_closed INTEGER NOT NULL DEFAULT 0 CHECK (jobs_closed >= 0),
  error TEXT,
  started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  finished_at TIMESTAMPTZ,
  CHECK (finished_at IS NULL OR finished_at >= started_at)
);

CREATE INDEX sync_runs_source_started_at_idx
  ON sync_runs (source_id, started_at DESC);

CREATE TABLE jobs (
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  source_id TEXT NOT NULL REFERENCES sources(id),
  external_id TEXT NOT NULL,
  title TEXT NOT NULL,
  description_html TEXT,
  apply_url TEXT NOT NULL,
  locations JSONB NOT NULL DEFAULT '[]'::JSONB
    CHECK (JSONB_TYPEOF(locations) = 'array'),
  workplace TEXT,
  employment_type TEXT,
  published_at TIMESTAMPTZ,
  first_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  closed_at TIMESTAMPTZ,
  last_seen_sync_run_id BIGINT REFERENCES sync_runs(id) ON DELETE SET NULL,
  raw JSONB NOT NULL,
  CONSTRAINT jobs_source_external_id_unique UNIQUE (source_id, external_id),
  CHECK (last_seen_at >= first_seen_at)
);

CREATE INDEX jobs_open_first_seen_at_idx
  ON jobs (first_seen_at DESC, id DESC)
  WHERE closed_at IS NULL;
