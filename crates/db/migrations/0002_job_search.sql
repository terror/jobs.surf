ALTER TABLE jobs
ADD COLUMN search_vector TSVECTOR
GENERATED ALWAYS AS (
  TO_TSVECTOR(
    'english'::REGCONFIG,
    title || ' ' || COALESCE(description_html, '')
  )
) STORED;

CREATE INDEX jobs_open_search_vector_idx
  ON jobs
  USING GIN (search_vector)
  WHERE closed_at IS NULL;
