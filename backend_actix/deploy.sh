#!/usr/bin/env bash
set -euo pipefail

SERVICE_NAME="${SERVICE_NAME:-backend-actix}"
PROJECT_ID="${PROJECT_ID:-$(gcloud config get-value project 2>/dev/null || true)}"
REGION="${REGION:-$(gcloud config get-value run/region 2>/dev/null || true)}"

if [[ -z "${PROJECT_ID}" || "${PROJECT_ID}" == "(unset)" ]]; then
  echo "PROJECT_ID is not set. Run: gcloud config set project YOUR_PROJECT_ID"
  exit 1
fi
if [[ -z "${REGION}" || "${REGION}" == "(unset)" ]]; then
  echo "REGION is not set. Run: gcloud config set run/region asia-southeast2"
  exit 1
fi

echo "Deploying: service=${SERVICE_NAME} project=${PROJECT_ID} region=${REGION}"

gcloud services enable run.googleapis.com cloudbuild.googleapis.com secretmanager.googleapis.com

# Deploy (public service)
gcloud run deploy "${SERVICE_NAME}" \
  --project "${PROJECT_ID}" \
  --region "${REGION}" \
  --source . \
  --no-invoker-iam-check

# Non-secret env vars
gcloud run services update "${SERVICE_NAME}" \
  --project "${PROJECT_ID}" \
  --region "${REGION}" \
  --set-env-vars \
RUST_ENV=production,\
HOST=0.0.0.0,\
JWT_ISSUER="${JWT_ISSUER:-}",\
JWT_ACCESS_EXPIRY="${JWT_ACCESS_EXPIRY:-3600}",\
JWT_REFRESH_EXPIRY="${JWT_REFRESH_EXPIRY:-604800}",\
JWT_VERIFICATION_EXPIRY="${JWT_VERIFICATION_EXPIRY:-86400}",\
SMTP_SERVER="${SMTP_SERVER:-}",\
SMTP_PORT="${SMTP_PORT:-587}",\
SMTP_USERNAME="${SMTP_USERNAME:-}",\
EMAIL_FROM="${EMAIL_FROM:-}",\
ARGON2_MEMORY_KIB="${ARGON2_MEMORY_KIB:-4096}",\
ARGON2_ITERATIONS="${ARGON2_ITERATIONS:-3}",\
ARGON2_PARALLELISM="${ARGON2_PARALLELISM:-1}",\
MULTIMEDIA_UPLOAD_BUCKET="${MULTIMEDIA_UPLOAD_BUCKET:-blogport-cms-upload}"

# Secrets -> env vars (expects secrets already exist)
gcloud run services update "${SERVICE_NAME}" \
  --project "${PROJECT_ID}" \
  --region "${REGION}" \
  --set-secrets \
DATABASE_URL=DATABASE_URL:latest,\
REDIS_URL=REDIS_URL:latest,\
JWT_SECRET=JWT_SECRET:latest,\
SMTP_PASSWORD=SMTP_PASSWORD:latest
