# Incident 0001 — Missing Environment Variables Causing DB Connection Failure

**Date:** 2026-01
**Component:** Local Actix backend + Dockerized PostgreSQL
**Severity:** Medium (development blocked, no data loss)

---

## Summary

After restarting the Actix backend (following stress tests), the application suddenly failed to connect to PostgreSQL, even though the database container was healthy and running.

The issue was **not caused by Docker, PostgreSQL, or Actix**, but by missing environment variables in the local shell.

---

## Symptoms

* Actix failed to start or lost its database connection
* `psql` failed when using `$DATABASE_URL`
* PostgreSQL container was running normally and accepting connections internally

Observed error when running:

```bash
psql "$DATABASE_URL" -c "select 1;"
```

Resulted in:

```text
psql: error: connection to server on socket "/tmp/.s.PGSQL.5432" failed: No such file or directory
	Is the server running locally and accepting connections on that socket?
```

Additional confirmation:

```bash
echo "$DATABASE_URL"
# (empty output)
```

---

## Root Cause

* The project stores configuration in `.env`
* Docker Compose **automatically loads `.env`** and injects it into containers
* The local shell **does NOT automatically load `.env`**
* Actix and `psql` (when run locally) depend on environment variables from the shell

After restarting Actix from a fresh terminal session, `DATABASE_URL` was **unset**.

When `psql` is executed without a host in the connection string, it falls back to **Unix socket mode**:

```text
/tmp/.s.PGSQL.5432
```

This does **not work with Dockerized PostgreSQL**, which is only reachable via TCP (`localhost:5432`).

Thus:

* PostgreSQL was healthy
* The client configuration was invalid due to missing environment variables

---

## Why the Issue Appeared “Suddenly”

The problem was latent and surfaced only after:

* restarting Actix
* opening a new terminal
* losing a previously sourced environment

Stress tests did not cause the issue directly — they **triggered a restart**, which exposed an implicit assumption that the environment was still loaded.

---

## Immediate Fix

Manually load environment variables into the shell:

```bash
set -a
source .env
set +a
```

After this:

```bash
psql "$DATABASE_URL" -c "select 1;"
```

worked correctly, and Actix could connect again.

---

## Long-Term Solution: `direnv` (Dev-Only)

To prevent this class of issue, the project now uses **direnv** to manage environment variables automatically.

### What direnv does

* Loads environment variables when entering the project directory
* Unloads them when leaving
* Prevents missing configuration after restarts or new terminals
* Keeps environment variables scoped to the project

### Project setup

Create a file named `.envrc` in the project root:

```bash
# .envrc
dotenv
```

Explanation:

* `dotenv` is a **direnv built-in directive**
* It tells direnv to read variables from `.env` and export them
* This replaces manual `source .env` steps

Then run once:

```bash
direnv allow
```

After this, environment variables are always available when working inside the project directory.

---

## Key Takeaway

This was **not a database or Docker failure**.

It was an **environment-loading issue caused by an implicit assumption**.

> If PostgreSQL is running but Actix or `psql` suddenly fails,
> always check:
>
> ```bash
> echo "$DATABASE_URL"
> ```

---

## Follow-up Improvements (Optional)

* Fail fast on startup if `DATABASE_URL` is missing
* Document `direnv` usage in `README.md`
* Add DB connection retry/backoff to Actix startup

---

This incident highlights the importance of **explicit environment management** when running services locally alongside Dockerized infrastructure.
