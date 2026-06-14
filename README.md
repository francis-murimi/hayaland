# Hayaland

A world-class Rust API built with **Actix Web** and **PostgreSQL**.

The codebase is organised as a Cargo workspace following **hexagonal/clean architecture** so it can grow exponentially without turning into a big ball of mud.

## Architecture

```text
crates/
├── domain/          # Entities, value objects, repository ports, domain errors (no DB/HTTP deps)
├── application/     # Use cases, DTOs, application errors, outbound ports (e.g. PasswordHasher)
├── infrastructure/  # PostgreSQL repositories, Argon2 hasher, config, telemetry, migrations
└── api/             # Actix routes, handlers, DTOs, error mapping, health endpoint
```

Dependency direction:

```text
api → application → domain
api → infrastructure → domain
infrastructure → application
```

## Quick start

### 1. PostgreSQL

You can use Docker:

```bash
docker compose up -d db
```

Or run a local cluster in the project directory (no Docker/sudo needed):

```bash
export PATH="/usr/lib/postgresql/18/bin:$PATH"
initdb -D .pgdata -U hayaland -A trust --locale=C --encoding=UTF8
pg_ctl -D .pgdata -l .pgdata/server.log -o "-p 5432 -c unix_socket_directories=/tmp" start
psql -U hayaland -h 127.0.0.1 -p 5432 -d postgres -c "CREATE DATABASE hayaland;"
psql -U hayaland -h 127.0.0.1 -p 5432 -d postgres -c "CREATE DATABASE hayaland_test;"
```

### 2. Environment

```bash
cp .env.example .env
# Edit .env if your Postgres URL differs.
```

### 3. Migrations

```bash
sqlx migrate run --database-url "postgres://hayaland@127.0.0.1:5432/hayaland"
```

### 4. Run

```bash
cargo run -p api
```

The server starts on `http://127.0.0.1:8080` by default.

## API

### Health

```bash
curl http://127.0.0.1:8080/api/v1/health
```

### Users

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/users` | Create a user |
| GET | `/api/v1/users` | List users (`page`, `per_page`, `active_only`) |
| GET | `/api/v1/users/{id}` | Get a user by ID |
| PATCH | `/api/v1/users/{id}` | Update email/username |
| DELETE | `/api/v1/users/{id}` | Soft-delete (sets `is_active = false`) |
| POST | `/api/v1/auth/login` | Login with email/password, returns JWT |

### Create user

```bash
curl -X POST http://127.0.0.1:8080/api/v1/users \
  -H "Content-Type: application/json" \
  -d '{"email":"alice@example.com","username":"alice","password":"password123"}'
```

Expected: `201 Created` with `{ "id": "..." }`.

Duplicate email/username returns `409 Conflict` with a structured error.
Invalid input returns `400 Bad Request` with field-level details.

### Login

```bash
curl -X POST http://127.0.0.1:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"alice@example.com","password":"password123"}'
```

Deactivated users receive `401 Unauthorized` with code `account_inactive`.

## Development

```bash
# Format & lint
cargo fmt
cargo clippy -- -D warnings

# Tests (requires DATABASE_URL to point at a Postgres server)
export DATABASE_URL="postgres://hayaland@127.0.0.1:5432/hayaland_test"
cargo test

# Prepare offline query metadata (commit the generated .sqlx/ directory)
cargo sqlx prepare --workspace
```

## Tech stack

- **Web:** Actix Web 4
- **Database:** PostgreSQL 16+ via sqlx (compile-time checked queries)
- **Migrations:** sqlx-cli
- **Password hashing:** Argon2id
- **Observability:** tracing + tracing-actix-web
- **Config:** `config` crate with `APP_*` environment variables
- **Secrets:** `secrecy`
