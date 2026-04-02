# scylladb-api

A PostgREST-style REST API for ScyllaDB. Automatically exposes every table in a configured keyspace (default: `public`) as a CRUD endpoint — no code changes needed when you add or modify tables.

## Base URL

```
http://localhost:8082
```

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `SCYLLA_HOST` | `127.0.0.1` | ScyllaDB host |
| `SCYLLA_PORT` | `9042` | ScyllaDB CQL port |
| `SCYLLA_USERNAME` | `scylla_app` | CQL username |
| `SCYLLA_PASSWORD` | `change_me_local` | CQL password |
| `SCYLLA_KEYSPACE` | `public` | Target keyspace |
| `API_HOST` | `127.0.0.1` | Bind address |
| `API_PORT` | `8082` | Bind port |
| `CORS_ORIGIN` | `http://localhost:3000` | Allowed CORS origin |

---

## Endpoints

### Health Check

```
GET /health
```

```json
{ "status": "ok", "service": "scylladb-api" }
```

---

### GET `/{table}` — Read rows

Returns rows from the table as a JSON array. All filters, projection, ordering, and pagination are controlled via query parameters.

```
GET /{table}?[select=...]&[col=op.value]&[order=...]&[limit=N]&[page_token=...]
```

#### Query Parameters

| Parameter | Description | Example |
|---|---|---|
| `select` | Comma-separated column projection | `select=id,name,email` |
| `{col}=op.value` | Filter on a column (see operators below) | `status=eq.active` |
| `order` | Comma-separated `col.asc` or `col.desc` | `order=created_at.desc,name.asc` |
| `limit` | Max rows to return (1–1000, default `100`) | `limit=20` |
| `page_token` | Cursor from `X-Next-Page-Token` for pagination | `page_token=ABcd...` |

#### Filter Operators

| Operator | Meaning | Example |
|---|---|---|
| `eq` | Equal | `age=eq.30` |
| `neq` | Not equal | `status=neq.deleted` |
| `gt` | Greater than | `score=gt.100` |
| `gte` | Greater than or equal | `score=gte.100` |
| `lt` | Less than | `price=lt.50` |
| `lte` | Less than or equal | `price=lte.50` |
| `in` | In a set | `id=in.(1,2,3)` |
| `is` | Is null | `deleted_at=is.null` |

> **Note:** Filtering on non-primary-key columns requires `ALLOW FILTERING` in CQL, which is appended automatically. For better performance, add a secondary index on frequently filtered columns.

#### Response Headers

| Header | Description |
|---|---|
| `Content-Range` | Row count in the current page (`0-N/N`) |
| `X-Next-Page-Token` | Base64 cursor — present only when more pages exist |
| `X-Warning` | `allow-filtering-applied` when a WHERE clause is used |

#### Examples

```bash
# All rows (default limit 100)
GET /users

# Select specific columns
GET /users?select=id,name,email

# Filter with operators
GET /users?status=eq.active
GET /users?age=gte.18&status=eq.active
GET /orders?total=gt.100&order=created_at.desc

# IN operator
GET /products?category=in.(shoes,bags,hats)

# Pagination — first page
GET /events?limit=20

# Pagination — next page (use token from X-Next-Page-Token header)
GET /events?limit=20&page_token=CmYKEgoEAAAB...
```

---

### POST `/{table}` — Insert rows

Insert a single object or an array of objects.

```
POST /{table}
Content-Type: application/json
```

#### Request Body

Single row:
```json
{ "id": "550e8400-e29b-41d4-a716-446655440000", "name": "Alice", "age": 30 }
```

Multiple rows:
```json
[
  { "id": "550e8400-e29b-41d4-a716-446655440000", "name": "Alice", "age": 30 },
  { "id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8", "name": "Bob",   "age": 25 }
]
```

#### Response

By default returns `201 Created` with an empty body.

To receive the inserted rows back, send:
```
Prefer: return=representation
```
Response will be `201 Created` with the inserted row(s) as JSON.

#### Examples

```bash
# Insert one row
curl -X POST http://localhost:8082/users \
  -H "Content-Type: application/json" \
  -d '{"id":"550e8400-e29b-41d4-a716-446655440000","name":"Alice","age":30}'

# Insert and get rows back
curl -X POST http://localhost:8082/users \
  -H "Content-Type: application/json" \
  -H "Prefer: return=representation" \
  -d '{"id":"550e8400-e29b-41d4-a716-446655440000","name":"Alice","age":30}'
```

---

### PATCH `/{table}?filter` — Update rows

Updates columns on all rows matching the filter(s). **At least one filter is required.**

```
PATCH /{table}?{col}=op.value
Content-Type: application/json
```

#### Request Body

JSON object mapping column names to new values:
```json
{ "status": "inactive", "updated_at": "2026-04-02T00:00:00Z" }
```

#### Response

`204 No Content` on success.

#### Examples

```bash
# Update a single row by primary key
curl -X PATCH "http://localhost:8082/users?id=eq.550e8400-e29b-41d4-a716-446655440000" \
  -H "Content-Type: application/json" \
  -d '{"name":"Alice Smith"}'

# Update multiple rows matching a filter
curl -X PATCH "http://localhost:8082/orders?status=eq.pending" \
  -H "Content-Type: application/json" \
  -d '{"status":"processing"}'
```

---

### DELETE `/{table}?filter` — Delete rows

Deletes all rows matching the filter(s). **At least one filter is required** to prevent accidental full-table deletes.

```
DELETE /{table}?{col}=op.value
```

#### Response

`204 No Content` on success.

#### Examples

```bash
# Delete by primary key
curl -X DELETE "http://localhost:8082/users?id=eq.550e8400-e29b-41d4-a716-446655440000"

# Delete all rows matching a filter
curl -X DELETE "http://localhost:8082/sessions?expires_at=lte.2026-01-01"
```

---

## Error Responses

All errors return a JSON body:

```json
{ "error": "description of the error" }
```

| Status | Cause |
|---|---|
| `400 Bad Request` | Invalid filter syntax, missing required params, bad JSON body |
| `404 Not Found` | Table does not exist in the configured keyspace |
| `500 Internal Server Error` | ScyllaDB query failure |

---

## Running Locally

```bash
# With Docker Compose (recommended)
docker compose up api

# Standalone
cd scylladb-api
SCYLLA_HOST=127.0.0.1 SCYLLA_USERNAME=cassandra SCYLLA_PASSWORD=cassandra \
SCYLLA_KEYSPACE=public API_PORT=8082 \
cargo run
```
