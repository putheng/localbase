# scylladb-meta

Rust metadata API for ScyllaDB (similar purpose to postgres-meta for Postgres).

Built with:
- actix-web
- scylla Rust driver

## Endpoints

- `GET /health`
- `GET /meta/keyspaces`
- `GET /meta/keyspaces/{keyspace}/tables`
- `GET /meta/keyspaces/{keyspace}/tables/{table}/columns`

## Environment

Copy `.env.example` to `.env` and update values:

```bash
cp .env.example .env
```

Required variables:
- `SCYLLA_HOST`
- `SCYLLA_PORT`
- `SCYLLA_USERNAME`
- `SCYLLA_PASSWORD`

Optional variables:
- `META_API_HOST` (default `127.0.0.1`)
- `META_API_PORT` (default `8080`)
- `CORS_ORIGIN` (default `http://localhost:3000`)

## Run

```bash
cargo run
```

## Test from terminal

```bash
curl http://127.0.0.1:8080/health
curl http://127.0.0.1:8080/meta/keyspaces
```
