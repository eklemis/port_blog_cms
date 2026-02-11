# Cloud Run Deployment: Public Service + App JWT Auth

This guide deploys the Rust **backend_actix** service to Google Cloud Run as a **public** HTTP service, protected by your **existing JWT auth** at the application layer.

It uses:
- Cloud Run (public access)
- Secret Manager (DATABASE_URL, REDIS_URL, JWT_SECRET, SMTP_PASSWORD)
- Normal environment variables for non-secrets

Using:
- Neon Postgres (remote)
- Upstash Redis (remote)

---

## What is the output

- A public Cloud Run URL like `https://<service>-<hash>-<region>.a.run.app`
- Cloud Run has:
  - **Secrets** injected as env vars: `DATABASE_URL`, `REDIS_URL`, `JWT_SECRET`, `SMTP_PASSWORD`
  - **Non-secrets** set as env vars: `JWT_ISSUER`, expiries, SMTP host/user, Argon2 settings, bucket name, etc.
- Your frontend calls the API normally and sends `Authorization: Bearer <JWT>` (no Google service-account key in browser code).

---

## Step 1 — Make the app Cloud Run compatible (required)

Cloud Run requires the container to listen on `0.0.0.0:$PORT` (PORT is provided by Cloud Run; usually 8080).

### 1.1 Update HOST/PORT reading

In the `start()` function, these mus applied:

```rust
let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
```

Why:
- Cloud Run does **not** set HOST.
- Cloud Run **does** set PORT.
- Defaulting makes local dev and Cloud Run both work.

Commit this change.

---

## Step 2 — Add Dockerfile (required)

Create `Dockerfile` in repo root:

```dockerfile
# ---- build stage ----
FROM rust:1.88 as builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY . .
RUN cargo build --release

# ---- runtime stage ----
FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/backend_actix /app/server
ENV HOST=0.0.0.0
ENV PORT=8080
EXPOSE 8080
CMD ["/app/server"]
```

---

## Step 3 — One-time GCP setup (do once per project)

### 3.1 Login + set project + set region

```bash
gcloud auth login
gcloud config set project YOUR_PROJECT_ID
gcloud config set run/region asia-southeast2
```

### 3.2 Enable required APIs

```bash
gcloud services enable run.googleapis.com cloudbuild.googleapis.com secretmanager.googleapis.com
```

---

## Step 4 — Run the bootstrap script (recommended)

The script:
1) Deploys Cloud Run service (public)
2) Detects the runtime service account (no guessing)
3) Creates/updates secrets (safe prompts)
4) Grants secret access to the runtime service account
5) Attaches secrets to Cloud Run as env vars
6) Sets non-secret env vars
7) Prints the service URL + log command

### 4.1 Run

From repo root:

```bash
chmod +x bootstrap_gcp.sh
./bootstrap_gcp.sh
```

You will be prompted for secrets (hidden input) and then for non-secrets (normal input).

### 4.2 Optional: skip prompts by exporting variables

```bash
export SERVICE_NAME="backend-actix"
export REGION="asia-southeast2"         # optional if already set in gcloud config
export PROJECT_ID="your-project-id"     # optional if already set in gcloud config

export DATABASE_URL="postgres://..."
export REDIS_URL="rediss://..."
export JWT_SECRET="..."
export SMTP_PASSWORD="..."

export JWT_ISSUER="Lotion"
export JWT_ACCESS_EXPIRY=3600
export JWT_REFRESH_EXPIRY=86400
export JWT_VERIFICATION_EXPIRY=86400
export SMTP_SERVER="smtp.example.com"
export SMTP_PORT=587
export SMTP_USERNAME="smtp-user"
export EMAIL_FROM="noreply@example.com"
export ARGON2_MEMORY_KIB=4096
export ARGON2_ITERATIONS=3
export ARGON2_PARALLELISM=1
export MULTIMEDIA_UPLOAD_BUCKET="your-bucket"

./bootstrap_gcp.sh
```

---

## Step 5 — Verify

### 5.1 Get service URL

```bash
gcloud run services describe backend-actix \
  --region "$(gcloud config get-value run/region)" \
  --format='value(status.url)'
```

### 5.2 Read logs

```bash
gcloud run services logs read backend-actix \
  --region "$(gcloud config get-value run/region)" \
  --limit 100
```

---

## Step 6 — Day-2 operations (repeatable)

### Deploy new code (keeps existing env + secrets)

```bash
gcloud run deploy backend-actix \
  --source . \
  --region "$(gcloud config get-value run/region)" \
  --no-invoker-iam-check
```

### Rotate a secret value (keeps mapping to latest)

```bash
echo -n 'new-value' | gcloud secrets versions add JWT_SECRET --data-file=-
```

No need to re-run `--set-secrets` if you mapped `:latest`.

---

## Important security note

Do **not** put any Google service account key in your frontend (browser). With Option A:
- Cloud Run is public
- Your API uses JWT auth
- Frontend sends `Authorization: Bearer <token>`
