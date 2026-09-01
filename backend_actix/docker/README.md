# Docker (local development)

This directory holds Compose files for **infrastructure only**.

- The Actix Web backend runs **directly on the host** (`cargo run`)
- Only PostgreSQL is containerised
- **Redis is not provided here** — the application requires it. See
  "Redis" below.

---

## PostgreSQL

Defined in `docker/postgres.compose.yml`. Container name: `cms-postgres`.

### Configuration

Compose resolves `env_file` **relative to the compose file**, not to the
directory you run the command from. So the settings go in `docker/.env`, not
in the project root:

```bash
cp docker/.env.example docker/.env
```

```env
POSTGRES_DB=cms
POSTGRES_USER=developer
POSTGRES_PASSWORD=change-me-locally
```

These configure the container itself. The application's own `DATABASE_URL`
lives in `backend_actix/.env` and must agree with them:

```env
DATABASE_URL=postgres://developer:change-me-locally@localhost:5432/cms
```

The `cms` database is created automatically on first startup. Changing
`POSTGRES_DB`, `POSTGRES_USER` or `POSTGRES_PASSWORD` afterwards has no effect
until the volume is removed — the values are only read when the data directory
is initialised.

---

## Start

Run from the `backend_actix` directory:

```bash
docker compose -f docker/postgres.compose.yml up -d
```

## Stop

```bash
docker compose -f docker/postgres.compose.yml down
```

## Reset (deletes all data)

```bash
docker compose -f docker/postgres.compose.yml down -v
```

## Open a psql shell

```bash
docker exec -it cms-postgres psql -d cms -U developer
```

## Verify the connection from the host

```bash
psql "$DATABASE_URL" -c "select 1;"
```

If this fails, the backend will not connect either. An empty `$DATABASE_URL`
is the usual cause — the shell does not load `.env` automatically the way
Compose does. See `docs/incidents/0001-env-loading.md`.

---

## Redis

The application requires `REDIS_URL`; it backs the refresh-token blacklist and
the rate-limit counters. There is no Compose file for it — run one directly:

```bash
docker run -d --name cms-redis -p 6379:6379 redis:7
redis-cli -u "$REDIS_URL" ping
```

---

## Notes

- PostgreSQL must expose port `5432`; the backend connects over TCP to
  `localhost:5432`.
- This setup is for **local development only**. Production uses managed
  Postgres and Redis, configured through Secret Manager — see
  `../build_steps.md`.
