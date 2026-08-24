# jobs.surf

`jobs.surf` aggregates jobs directly from company career pages and exposes them
through a read-only HTTP API and React application.

The backend is a Rust binary backed by PostgreSQL. The frontend lives in
`www/` and uses Bun, Vite, React, TypeScript, Tailwind CSS, shadcn, and a client
generated from the backend's OpenAPI document.

## Development

Requirements:

- Rust with Cargo
- Docker and Docker Compose
- Bun 1.3.14
- `just` for the convenience recipes

Start PostgreSQL:

```console
just services
```

Update `config.toml` with a real source, then synchronize it:

```console
cargo run -- sync --source acme-careers
```

Run the API and frontend in separate terminals:

```console
cargo run -- serve
bun install --cwd www
bun run --cwd www dev
```

The frontend is available at <http://127.0.0.1:5173>. Vite proxies API requests
to <http://127.0.0.1:3000>.

## HTTP API

The current read API provides:

- `GET /healthz`
- `GET /v1/jobs`
- `GET /v1/jobs/{id}`
- `GET /v1/sources`
- `GET /docs`

`GET /v1/jobs` supports `cursor`, `limit`, `query`, `remote`, and `source`
parameters. Results contain open jobs in newest-first order.

## Generated Client

Utoipa annotations in the Rust handlers are the source of truth. Regenerate the
committed OpenAPI document and TypeScript SDK after changing the HTTP contract:

```console
just www-generate
```

CI fails when `openapi/jobs-surf.json` or `www/src/client/` is stale.

## Checks

Run the backend and frontend checks with:

```console
just all
```

See [the architecture document](docs/architecture.md) for component boundaries,
sync semantics, persistence, and deployment assumptions.
