Below is a **rewritten README** with a **security-first, socially aware mindset**, written for developers who appreciate clarity without oversharing.
It explains *what the tool is for*, *what it is not*, and *how to use it safely*—without revealing operational details that could be abused.

---

# Test Utilities Service

A **test-only helper service** designed to support automated backend testing in environments where production-grade security constraints intentionally limit direct access.

This service exists to **remove manual testing steps** while **preserving the integrity of real application logic**.

It is not part of the application runtime and must never be treated as such.

---

## Why this exists

Production systems correctly restrict access to sensitive operations such as:

* issuing verification tokens
* generating expired or invalid authentication tokens
* modifying user lifecycle flags
* deleting user data arbitrarily

These restrictions protect real users—but they also make **repeatable automated testing difficult**.

This project provides a **separate, isolated utility service** that enables those operations **only in controlled test environments**, without weakening the production system itself.

---

## What this service does

At a high level, this service:

* Generates authentication tokens with **explicit and predictable behavior**
* Produces random credentials for test account creation
* Cleans up test data in a **safe, transactional** way
* Actively prevents execution in production environments

All behaviors are intentionally scoped to testing needs.

---

## What this service does *not* do

* It does **not** replace or proxy the real backend
* It does **not** contain business logic
* It does **not** share code with production services
* It does **not** bypass production security controls

Its sole purpose is to **support testing of the real system**, not to emulate it.

---

## Security posture (important)

This service is intentionally powerful and intentionally restricted.

To reduce the risk of misuse:

* It refuses to operate in production environments
* It exposes a `/health` endpoint that fails loudly if misconfigured
* It is designed to run on isolated ports and networks
* It avoids sharing internal implementation details of the real system

If this service ever becomes reachable by untrusted users, it should be considered compromised by definition.

---

## How it fits into a test workflow

```
Tests / Postman / CI
        │
        ▼
Test Utilities Service
        │
        ├── Generates controlled inputs (tokens, credentials)
        ├── Performs test data cleanup
        ▼
Real Backend (unchanged, secure)
```

The real backend remains the **source of truth**.
This service only provides **inputs** and **cleanup** to support testing.

---

## Installation

Install dependencies:

```bash
bun install
```

---

## Running the service

Start the test utility service:

```bash
bun start
```

The service will refuse to run or report unhealthy status if it detects a production environment.

---

## Runtime requirements

* Bun v1.1.8 or newer
* PostgreSQL (for test data cleanup)
* Environment variables for:

  * JWT signing secret (must match the backend under test)
  * Database connection
  * `NODE_ENV` (must not be `production`)

---

## Design principles

* **Isolation over reuse**
  Duplication is preferred to coupling with production code.

* **Determinism over realism**
  Tests need predictable outcomes, not perfect simulations.

* **Explicit over implicit**
  Unsafe operations are intentionally visible and clearly named.

* **Fail loudly**
  Misconfiguration is treated as an error, not a warning.

---

## Intended audience

This project is intended for:

* backend developers
* QA engineers
* CI pipelines
* integration testing environments

It is not intended for end users or production systems.

---

## Final note

This service exists to make testing **faster, cleaner, and more reliable**—
without compromising the security or integrity of the real application.

Used correctly, it disappears into the background of your workflow.
Used incorrectly, it should fail loudly.

---

Built with **Bun**, a fast all-in-one JavaScript runtime.
[https://bun.sh](https://bun.sh)
