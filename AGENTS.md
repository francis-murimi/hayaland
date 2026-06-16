# Agent Notes for `hayaland`

## Project type
Rust workspace (Actix Web + PostgreSQL via sqlx) using a hexagonal/clean architecture.

## Architecture rules
- `crates/domain` has no dependencies on web, DB, or configuration crates.
- `crates/application` depends only on `domain`.
- `crates/infrastructure` implements ports defined in `domain` and `application`.
- `crates/api` wires everything together and exposes the HTTP server.
- All public errors are typed with `thiserror`; API layer maps them to HTTP.

## Local development (no Docker required)
A PostgreSQL cluster is managed inside the project directory so you do not need `docker` or `sudo`.
The project standardises on port **5432**; make sure no other Postgres server is listening on that port before starting the local cluster.

```bash
# One-time init
export PATH="/usr/lib/postgresql/18/bin:$PATH"
initdb -D .pgdata -U hayaland -A trust --locale=C --encoding=UTF8

# Start the server
pg_ctl -D .pgdata -l .pgdata/server.log -o "-p 5432 -c unix_socket_directories=/tmp" start

# Create databases
psql -U hayaland -h 127.0.0.1 -p 5432 -d postgres -c "CREATE DATABASE hayaland;"
psql -U hayaland -h 127.0.0.1 -p 5432 -d postgres -c "CREATE DATABASE hayaland_test;"

# Run migrations
sqlx migrate run --database-url "postgres://hayaland@127.0.0.1:5432/hayaland"

# Build / run
cargo build
cargo run -p api
```

If you prefer Docker:

```bash
docker compose up -d db
sqlx migrate run
cargo run -p api
```

## Environment variables
Copy `.env.example` to `.env` and adjust values. `DATABASE_URL` is used by sqlx macros and tests; the running API uses `APP_DATABASE__URL` when present.

A default test `DATABASE_URL` is also provided in `.cargo/config.toml` so `cargo test` works without manually exporting the variable. You can override it by setting `DATABASE_URL` in your shell or `.env` file.

## Useful commands
- `cargo fmt --check && cargo clippy -- -D warnings`
- `cargo test`
- `cargo sqlx prepare --workspace` (commits offline query metadata to `.sqlx/`)
- `sqlx migrate add <name>`

## CI / GitHub
A GitHub Actions workflow is provided in `.github/workflows/ci.yml`. It runs formatting, clippy, migrations, tests, and an offline sqlx metadata check against a PostgreSQL service.

## Publishing checklist
- Copy `.env.example` to `.env` and replace secrets; never commit `.env`.
- Ensure `.sqlx/` is up to date: `cargo sqlx prepare --workspace`.
- Ensure migrations are idempotent and backwards-compatible.
- Do not commit `.pgdata/` or `target/` (both are `.gitignore`d).
- Verify CI passes locally before pushing.

## Migrations
All migrations live in `migrations/`. Keep them idempotent and backwards-compatible for a zero-downtime deployment.
