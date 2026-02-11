#!/usr/bin/env bash
set -euo pipefail

# ============================================================
# Cloud Run Deploy Script
#
# What it does, in order:
# 1) Ensure required APIs are enabled
# 2) Prompt for secrets and create in Secret Manager
# 3) Prompt for non-secret env vars and resource limits
# 4) Grant Secret Manager access to Cloud Run service account
# 5) Deploy Cloud Run service with image + all secrets + env vars
# 6) Print service URL + log tail command
#
# Run this every time you need to update configuration
# ============================================================

SERVICE_NAME="${SERVICE_NAME:-backend-actix}"
PROJECT_ID="${PROJECT_ID:-$(gcloud config get-value project 2>/dev/null || true)}"
REGION="${REGION:-$(gcloud config get-value run/region 2>/dev/null || true)}"
IMAGE_TAG="${IMAGE_TAG:-latest}"
IMAGE_NAME="gcr.io/${PROJECT_ID}/${SERVICE_NAME}:${IMAGE_TAG}"

die() { echo "ERROR: $*" >&2; exit 1; }

[[ -n "${PROJECT_ID}" && "${PROJECT_ID}" != "(unset)" ]] || die "PROJECT_ID not set. Run: gcloud config set project YOUR_PROJECT_ID (or export PROJECT_ID=...)"
[[ -n "${REGION}" && "${REGION}" != "(unset)" ]] || die "REGION not set. Run: gcloud config set run/region asia-southeast2 (or export REGION=...)"

command -v gcloud >/dev/null 2>&1 || die "gcloud not found in PATH"

echo "==> Project : ${PROJECT_ID}"
echo "==> Region  : ${REGION}"
echo "==> Service : ${SERVICE_NAME}"
echo "==> Image   : ${IMAGE_NAME}"
echo

# Check if image exists
if ! gcloud container images describe "${IMAGE_NAME}" --project "${PROJECT_ID}" >/dev/null 2>&1; then
  echo "WARNING: Image ${IMAGE_NAME} not found!"
  echo "Run ./build.sh first to build the image."
  read -p "Continue anyway? (y/N): " confirm
  [[ "${confirm}" =~ ^[Yy]$ ]] || exit 1
fi

echo "==> Enabling APIs (safe to re-run)..."
gcloud services enable \
  run.googleapis.com \
  secretmanager.googleapis.com \
  --project "${PROJECT_ID}" >/dev/null

# ------------------------------------------------------------
# Secret helpers
# ------------------------------------------------------------
secret_exists() {
  local name="$1"
  gcloud secrets describe "$name" --project "${PROJECT_ID}" >/dev/null 2>&1
}

create_or_update_secret_from_stdin() {
  local secret_name="$1"
  if secret_exists "${secret_name}"; then
    echo "==> Secret ${secret_name} exists -> adding new version"
    gcloud secrets versions add "${secret_name}" \
      --project "${PROJECT_ID}" \
      --data-file=-
  else
    echo "==> Creating secret ${secret_name}"
    gcloud secrets create "${secret_name}" \
      --project "${PROJECT_ID}" \
      --replication-policy="automatic" \
      --data-file=-
  fi
}

prompt_secret() {
  local var_name="$1"
  local prompt_text="$2"
  local value=""

  if [[ -n "${!var_name:-}" ]]; then
    value="${!var_name}"
  else
    read -s -p "${prompt_text}: " value
    echo
  fi
  printf "%s" "${value}"
}

prompt_nonsecret() {
  local var_name="$1"
  local prompt_text="$2"
  local default_value="$3"
  local value=""

  if [[ -n "${!var_name:-}" ]]; then
    value="${!var_name}"
  else
    read -p "${prompt_text} [default: ${default_value}]: " value
    if [[ -z "${value}" ]]; then
      value="${default_value}"
    fi
  fi
  printf "%s" "${value}"
}

# ------------------------------------------------------------
# Step 1: Create/update secrets
# ------------------------------------------------------------
echo "==> Creating/updating secrets (input hidden)..."
prompt_secret DATABASE_URL "DATABASE_URL (Neon Postgres)" | create_or_update_secret_from_stdin DATABASE_URL
prompt_secret REDIS_URL "REDIS_URL (Upstash Redis, usually rediss://...)" | create_or_update_secret_from_stdin REDIS_URL
prompt_secret JWT_SECRET "JWT_SECRET" | create_or_update_secret_from_stdin JWT_SECRET
prompt_secret SMTP_PASSWORD "SMTP_PASSWORD" | create_or_update_secret_from_stdin SMTP_PASSWORD
echo

# ------------------------------------------------------------
# Step 2: Prompt for non-secret env vars
# ------------------------------------------------------------
echo "==> Configuring non-secret env vars..."

RUST_ENV_VAL="$(prompt_nonsecret RUST_ENV "RUST_ENV" "production")"
CPU_VAL="$(prompt_nonsecret CPU "CPU (1, 2, 4, 6, 8)" "2")"
MEMORY_VAL="$(prompt_nonsecret MEMORY "MEMORY (512Mi, 1Gi, 2Gi, 4Gi, 8Gi)" "1Gi")"
JWT_ISSUER_VAL="$(prompt_nonsecret JWT_ISSUER "JWT_ISSUER" "Lotion")"
JWT_ACCESS_EXPIRY_VAL="$(prompt_nonsecret JWT_ACCESS_EXPIRY "JWT_ACCESS_EXPIRY (seconds)" "3600")"
JWT_REFRESH_EXPIRY_VAL="$(prompt_nonsecret JWT_REFRESH_EXPIRY "JWT_REFRESH_EXPIRY (seconds)" "86400")"
JWT_VERIFICATION_EXPIRY_VAL="$(prompt_nonsecret JWT_VERIFICATION_EXPIRY "JWT_VERIFICATION_EXPIRY (seconds)" "86400")"

SMTP_SERVER_VAL="$(prompt_nonsecret SMTP_SERVER "SMTP_SERVER" "")"
SMTP_PORT_VAL="$(prompt_nonsecret SMTP_PORT "SMTP_PORT" "587")"
SMTP_USERNAME_VAL="$(prompt_nonsecret SMTP_USERNAME "SMTP_USERNAME" "")"
EMAIL_FROM_VAL="$(prompt_nonsecret EMAIL_FROM "EMAIL_FROM" "")"

ARGON2_MEMORY_KIB_VAL="$(prompt_nonsecret ARGON2_MEMORY_KIB "ARGON2_MEMORY_KIB" "4096")"
ARGON2_ITERATIONS_VAL="$(prompt_nonsecret ARGON2_ITERATIONS "ARGON2_ITERATIONS" "3")"
ARGON2_PARALLELISM_VAL="$(prompt_nonsecret ARGON2_PARALLELISM "ARGON2_PARALLELISM" "1")"

MULTIMEDIA_UPLOAD_BUCKET_VAL="$(prompt_nonsecret MULTIMEDIA_UPLOAD_BUCKET "MULTIMEDIA_UPLOAD_BUCKET" "")"

# Cloud Run: bind 0.0.0.0; PORT is injected by Cloud Run automatically.
HOST_VAL="0.0.0.0"

echo

# ------------------------------------------------------------
# Step 3: Grant Secret Manager access to default compute SA
# ------------------------------------------------------------
echo "==> Granting Secret Manager access to Cloud Run service account..."

# Get project number (required for service account name)
PROJECT_NUMBER="$(gcloud projects describe "${PROJECT_ID}" --format='value(projectNumber)' 2>/dev/null)"
[[ -n "${PROJECT_NUMBER}" ]] || die "Could not determine project number for ${PROJECT_ID}"

DEFAULT_COMPUTE_SA="${PROJECT_NUMBER}-compute@developer.gserviceaccount.com"
echo "==> Service account: ${DEFAULT_COMPUTE_SA}"

# Grant to default compute service account (used by Cloud Run by default)
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
  --member="serviceAccount:${DEFAULT_COMPUTE_SA}" \
  --role="roles/secretmanager.secretAccessor" \
  >/dev/null

echo "==> Granted secretAccessor role"
echo

# ------------------------------------------------------------
# Step 4: Deploy Cloud Run service with secrets + env vars
# ------------------------------------------------------------
echo "==> Deploying Cloud Run service (public) with all config..."
gcloud run deploy "${SERVICE_NAME}" \
  --project "${PROJECT_ID}" \
  --region "${REGION}" \
  --image "${IMAGE_NAME}" \
  --cpu "${CPU_VAL}" \
  --memory "${MEMORY_VAL}" \
  --allow-unauthenticated \
  --set-secrets \
DATABASE_URL=DATABASE_URL:latest,\
REDIS_URL=REDIS_URL:latest,\
JWT_SECRET=JWT_SECRET:latest,\
SMTP_PASSWORD=SMTP_PASSWORD:latest \
  --set-env-vars \
RUST_ENV="${RUST_ENV_VAL}",\
HOST="${HOST_VAL}",\
JWT_ISSUER="${JWT_ISSUER_VAL}",\
JWT_ACCESS_EXPIRY="${JWT_ACCESS_EXPIRY_VAL}",\
JWT_REFRESH_EXPIRY="${JWT_REFRESH_EXPIRY_VAL}",\
JWT_VERIFICATION_EXPIRY="${JWT_VERIFICATION_EXPIRY_VAL}",\
SMTP_SERVER="${SMTP_SERVER_VAL}",\
SMTP_PORT="${SMTP_PORT_VAL}",\
SMTP_USERNAME="${SMTP_USERNAME_VAL}",\
EMAIL_FROM="${EMAIL_FROM_VAL}",\
ARGON2_MEMORY_KIB="${ARGON2_MEMORY_KIB_VAL}",\
ARGON2_ITERATIONS="${ARGON2_ITERATIONS_VAL}",\
ARGON2_PARALLELISM="${ARGON2_PARALLELISM_VAL}",\
MULTIMEDIA_UPLOAD_BUCKET="${MULTIMEDIA_UPLOAD_BUCKET_VAL}"

echo
echo "==> ✓ Deployment complete!"
echo
echo "==> Service URL:"
gcloud run services describe "${SERVICE_NAME}" \
  --project "${PROJECT_ID}" \
  --region "${REGION}" \
  --format='value(status.url)'

echo
echo "==> If anything fails, read logs:"
echo "gcloud run services logs read \"${SERVICE_NAME}\" --project \"${PROJECT_ID}\" --region \"${REGION}\" --limit 100"
