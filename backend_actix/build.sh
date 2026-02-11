#!/usr/bin/env bash
set -euo pipefail

# ============================================================
# Cloud Run Build Script
#
# Builds and pushes container image to Google Container Registry
# Run this only when your code changes
# ============================================================

SERVICE_NAME="${SERVICE_NAME:-backend-actix}"
PROJECT_ID="${PROJECT_ID:-$(gcloud config get-value project 2>/dev/null || true)}"
IMAGE_TAG="${IMAGE_TAG:-latest}"
IMAGE_NAME="gcr.io/${PROJECT_ID}/${SERVICE_NAME}:${IMAGE_TAG}"

die() { echo "ERROR: $*" >&2; exit 1; }

[[ -n "${PROJECT_ID}" && "${PROJECT_ID}" != "(unset)" ]] || die "PROJECT_ID not set. Run: gcloud config set project YOUR_PROJECT_ID"

command -v gcloud >/dev/null 2>&1 || die "gcloud not found in PATH"

echo "==> Project : ${PROJECT_ID}"
echo "==> Service : ${SERVICE_NAME}"
echo "==> Image   : ${IMAGE_NAME}"
echo

echo "==> Enabling required APIs..."
gcloud services enable \
  cloudbuild.googleapis.com \
  containerregistry.googleapis.com \
  --project "${PROJECT_ID}" >/dev/null

echo "==> Building container image with Cloud Build..."
gcloud builds submit \
  --tag "${IMAGE_NAME}" \
  --project "${PROJECT_ID}" \
  --machine-type=e2-highcpu-8 \
  .

echo
echo "==> ✓ Build complete!"
echo "==> Image: ${IMAGE_NAME}"
echo
echo "==> Next step: Run ./deploy.sh to deploy this image to Cloud Run"
