# konfig

A production-grade **Settings Management REST API** built in Rust. Stores typed application settings in PostgreSQL with a flexible JSONB payload for arbitrary key-value configuration.

## Tech stack

- **[axum](https://github.com/tokio-rs/axum)** — async web framework
- **[sqlx](https://github.com/launchbrary/sqlx)** — async, compile-checked PostgreSQL client
- **[serde](https://serde.rs)** — JSON serialization
- **[validator](https://github.com/Keats/validator)** — request validation
- **[tracing](https://github.com/tokio-rs/tracing)** — structured logging

> **Phase 2** (planned): Redis caching layer via `fred`.

## Data model

Each setting has a unique string key, a type enum, and a flexible `JSONB` value field.

| Column | Type | Description |
|---|---|---|
| `id` | UUID | Auto-generated primary key |
| `key` | TEXT UNIQUE | Identifier, e.g. `"feature.dark_mode"` |
| `setting_type` | TEXT | `feature_flag` \| `limit` \| `appearance` \| `integration` \| `custom` |
| `value` | JSONB | Arbitrary JSON payload |
| `description` | TEXT | Optional human-readable description |
| `is_active` | BOOLEAN | Default `true` |
| `created_at` | TIMESTAMPTZ | Set on insert |
| `updated_at` | TIMESTAMPTZ | Updated on every PATCH |

## API

| Method | Path | Description |
|---|---|---|
| `GET` | `/health` | Health check (DB connectivity) |
| `POST` | `/settings` | Create a setting |
| `GET` | `/settings` | List settings (filterable, paginated) |
| `GET` | `/settings/:key` | Fetch a setting by key |
| `PATCH` | `/settings/:key` | Partially update a setting |
| `DELETE` | `/settings/:key` | Delete a setting |

### Query parameters for `GET /settings`

| Parameter | Type | Description |
|---|---|---|
| `type` | string | Filter by `setting_type` |
| `active` | bool | Filter by `is_active` |
| `page` | int | Page number (default: 1) |
| `per_page` | int | Results per page (default: 20, max: 100) |

### Error responses

All errors return structured JSON:

```json
{
  "error": "not_found",
  "message": "Setting 'feature.dark_mode' does not exist"
}
```

| Status | `error` field | Cause |
|---|---|---|
| 404 | `not_found` | Key does not exist |
| 409 | `conflict` | Duplicate key on create |
| 422 | `validation_error` | Invalid request payload |
| 500 | `database_error` | Unexpected database failure |

## Getting started

### Prerequisites

- Rust 1.75+
- Docker (for the local database)
- [`sqlx-cli`](https://github.com/launchbrary/sqlx/tree/master/sqlx-cli) (optional, for manual migrations)

```bash
cargo install sqlx-cli --no-default-features --features native-tls,postgres
```

### 1. Start the database

```bash
docker compose up -d
```

### 2. Configure environment

```bash
cp .env.example .env
```

The defaults in `.env.example` match the Docker Compose credentials and are ready to use as-is.

### 3. Run the API

```bash
cargo run
```

Migrations run automatically on startup. The server listens on `http://localhost:8080`.

## Usage examples

### Create a setting

```bash
curl -s -X POST http://localhost:8080/settings \
  -H "Content-Type: application/json" \
  -d '{
    "key": "feature.dark_mode",
    "setting_type": "feature_flag",
    "value": { "enabled": true, "rollout_percentage": 50 },
    "description": "Enable dark mode UI",
    "is_active": true
  }' | jq
```

### Fetch a setting

```bash
curl -s http://localhost:8080/settings/feature.dark_mode | jq
```

### List settings with filters

```bash
# All active feature flags, page 1
curl -s "http://localhost:8080/settings?type=feature_flag&active=true&page=1&per_page=10" | jq
```

### Update a setting (partial)

```bash
curl -s -X PATCH http://localhost:8080/settings/feature.dark_mode \
  -H "Content-Type: application/json" \
  -d '{ "value": { "enabled": false }, "is_active": false }' | jq
```

### Delete a setting

```bash
curl -s -X DELETE http://localhost:8080/settings/feature.dark_mode
# 204 No Content
```

### Health check

```bash
curl -s http://localhost:8080/health | jq
# { "status": "ok", "db": "ok" }
```

## Running tests

Tests use `#[sqlx::test]` — each test gets an isolated database with migrations applied automatically. A running PostgreSQL instance is required.

```bash
# With Docker Compose running:
DATABASE_URL=postgres://konfig:konfig@localhost:5432/settings_db cargo test
```

The test suite covers:

- Create → fetch → update → delete lifecycle
- Duplicate key returns `409 Conflict`
- Missing key returns `404 Not Found`
- List filtering by type and active status

## Project structure

```
konfig/
├── docker-compose.yml
├── .env.example
├── migrations/
│   └── 001_create_settings.sql
└── src/
    ├── main.rs          # entry point
    ├── lib.rs           # public module exports
    ├── config.rs        # environment configuration
    ├── error.rs         # AppError + IntoResponse
    ├── state.rs         # AppState (db pool)
    ├── routes.rs        # router definition
    ├── models/
    │   └── setting.rs   # Setting, SettingType, request/response types
    ├── db/
    │   └── settings.rs  # sqlx queries
    └── handlers/
        ├── mod.rs       # health handler
        └── settings.rs  # CRUD handlers
```

## Environment variables

| Variable | Default | Description |
|---|---|---|
| `APP_PORT` | `8080` | Listen port |
| `APP_ENV` | `development` | Environment name |
| `DATABASE_URL` | — | PostgreSQL connection string |
| `DATABASE_MAX_CONNECTIONS` | `10` | Connection pool size |
| `RUST_LOG` | `info` | Log filter (e.g. `konfig=debug`) |
| `REDIS_URL` | — | Redis URL (Phase 2) |
| `CACHE_TTL_SECS` | `300` | Cache TTL in seconds (Phase 2) |
