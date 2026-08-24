# jobs.surf Architecture

## System

```text
Greenhouse --\
Ashby -------+--> compiled adapters --> sync orchestration --> PostgreSQL
Other -------/                                              |
                                                            v
React application --> generated HTTP client --> Axum read API
```

The repository is a Cargo workspace with one executable program and three
focused library crates. The React application is an independent Bun package.

```text
src/                 CLI, sync orchestration, Axum API, OpenAPI
crates/model/        canonical provider-neutral data
crates/adapter/      provider HTTP clients and normalization
crates/db/           PostgreSQL repository and migrations
openapi/             generated API contract snapshot
www/                 Vite React application and generated TypeScript client
```

This reflects the current repository. Earlier guidance described a single,
dependency-free crate and a Greenhouse-only milestone; that is no longer the
implemented shape.

## Boundaries

Adapters own provider DTOs, HTTP request details, and translation to
`JobSnapshot`. Provider-specific types remain private to their adapter.

The root application owns source selection and synchronization order. It starts
a sync run before fetching, validates the complete normalized snapshot, and
then either completes or fails the run.

The database crate owns SQL, migrations, transactions, job lifecycle state,
and cursor pagination. Axum handlers do not issue SQL directly.

The API owns HTTP extraction and public response DTOs. Database records and raw
provider payloads are not serialized directly.

The React application knows only the HTTP contract. It does not import provider
configuration or reproduce Rust DTOs by hand.

## Adapters

Adapters are compiled into the binary and selected from typed TOML
configuration. The current providers are Ashby, Breezy, Comeet, Greenhouse,
Lever, Personio, Recruitee, Teamtailor, and Workable.

An adapter fetch returns one complete snapshot. Greenhouse verifies the
provider's reported total before normalization. A request, decode, validation,
or completeness failure never reaches the transaction that closes missing
jobs.

Provider responses are retained as private raw JSON for debugging. Normalized
fields use optional values rather than placeholder strings.

## Synchronization

`jobs-surf sync --source <id>` performs this sequence:

1. Parse typed source configuration.
2. Reject missing, duplicate, or disabled source selections.
3. Upsert source metadata and commit a running sync record.
4. Fetch and normalize the complete provider snapshot.
5. Validate identifiers, titles, and duplicate external IDs.
6. Begin the completion transaction.
7. Upsert all jobs by `(source_id, external_id)` and reopen reappearing jobs.
8. Close open jobs not seen in this successful snapshot.
9. Mark the run successful with counts and commit atomically.

Any error after step 3 marks the run failed. Failed fetches and invalid or
partial snapshots do not modify jobs. A run cannot overwrite a source after a
newer successful run has completed.

Synchronization is currently invoked one source at a time. Scheduling,
config-wide reconciliation, automatic retries, and disabling jobs from removed
sources remain external operational concerns.

## Persistence

PostgreSQL contains `sources`, `jobs`, and `sync_runs`. Jobs use internal
`BIGINT` identity values and a unique `(source_id, external_id)` constraint.
Public JSON represents the internal ID as a decimal string.

Job rows track first seen, last seen, closed time, and the last successful sync
identity. Locations remain JSONB. Raw provider objects remain private JSONB.

Search uses a stored generated `TSVECTOR` over title and description with a
partial GIN index for open jobs. Search filters chronologically ordered results
rather than introducing relevance ordering, so the existing `(first_seen_at,
id)` cursor remains stable.

## API

The API is read-only:

```text
GET /healthz
GET /v1/jobs
GET /v1/jobs/{id}
GET /v1/sources
GET /docs
```

Open jobs are returned by default. Job listing filters are conjunctive and are
applied in PostgreSQL before keyset pagination. Detail requests return only
open jobs. Source responses omit persisted adapter configuration.

Provider HTML is sanitized while mapping repository records to public response
DTOs. The original provider payload is never exposed.

## Contract Generation

Utoipa annotations on handlers and response DTOs are the contract source. The
`jobs-surf openapi --output <path>` command serializes the contract without
connecting to PostgreSQL.

`openapi/jobs-surf.json` is committed. `@hey-api/openapi-ts` generates the
Fetch-based TypeScript SDK under `www/src/client/`. CI regenerates both and
fails on any diff, then typechecks and builds the application with Bun.

## Frontend

`www/` is a Vite React and TypeScript application styled with Tailwind CSS and
shadcn components. It uses relative API URLs. During development, Vite proxies
`/v1`, `/healthz`, and `/docs` to Axum on port 3000, so development-only CORS is
not required.

The Rust binary does not embed static frontend assets. Production can deploy
`www/dist/` separately behind the same origin or package it beside the binary
with a reverse proxy. Ordinary Cargo builds remain independent of Bun.

## Testing

Adapter fixture tests cover provider normalization and unknown fields. The
Greenhouse integration test runs a local Axum provider, executes initial,
changed, and failed snapshots through production sync orchestration, verifies
PostgreSQL lifecycle state, and queries the production API router.

Database integration tests use isolated PostgreSQL databases. API tests invoke
the Axum router directly through `tower::ServiceExt`. CI also runs strict
Clippy, rustfmt, coverage, frontend linting, TypeScript checks, a production
Vite build, dependency audit, and generated-contract drift checks.

Live provider endpoints are not used by automated tests.
