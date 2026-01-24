# Docker (Local Development)

This directory contains Docker Compose files for **infrastructure only**.

- The Actix Web backend runs **directly on the host**
- Only PostgreSQL is containerized

---

## PostgreSQL

Managed via:

```

docker/postgres.compose.yml

````

### Required environment variables (`.env` in project root)

```env
POSTGRES_DB=cms
POSTGRES_USER=postgres
POSTGRES_PASSWORD=postgres
DATABASE_URL=postgres://postgres:postgres@localhost:5432/cms
````

The database `cms` is created automatically on first startup.

---

## Start PostgreSQL

From project root:

```bash
docker compose -f docker/postgres.compose.yml up -d
```

---

## Stop PostgreSQL

```bash
docker compose -f docker/postgres.compose.yml down
```

---

## Reset Database (delete all data)

```bash
docker compose -f docker/postgres.compose.yml down -v
```

---

## Verify Connection (from host)

```bash
psql "$DATABASE_URL" -c "select 1;"
```

If this fails, Actix will not connect.

---

## Notes

* PostgreSQL **must expose a port** (`5432`)
* Actix connects via TCP (`localhost:5432`)
* This setup is for **local development only**
