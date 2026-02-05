#!/bin/bash
set -euo pipefail

# =============================================================================
# Configuration - Update these values for your project
# =============================================================================
if [[ -f .env ]]; then
    source .env
fi

PROJECT_ID="${PROJECT_ID:-$(gcloud config get-value project)}"
REGION="${REGION:-$(gcloud config get-value run/region)}"
SERVICE_NAME="${SERVICE_NAME:-image-processor-function}"
BUCKET_NAME="${BUCKET_NAME:?Error: BUCKET_NAME not set}"

# Artifact Registry repository (will be created if it doesn't exist)
REPO_NAME="cloud-run-functions"
IMAGE_URI="${REGION}-docker.pkg.dev/${PROJECT_ID}/${REPO_NAME}/${SERVICE_NAME}:latest"

# =============================================================================
# Pre-flight checks
# =============================================================================
echo "📋 Configuration:"
echo "   Project:  ${PROJECT_ID}"
echo "   Region:   ${REGION}"
echo "   Service:  ${SERVICE_NAME}"
echo "   Bucket:   ${BUCKET_NAME}"
echo ""

if [[ "${PROJECT_ID}" == "your-project-id" ]]; then
    echo "❌ Error: PROJECT_ID not set and no gcloud default"
    echo "   export PROJECT_ID=your-actual-project-id"
    exit 1
fi

if [[ "${BUCKET_NAME}" == "your-bucket-name" ]]; then
    echo "❌ Error: Please set BUCKET_NAME environment variable"
    echo "   export BUCKET_NAME=your-actual-bucket-name"
    exit 1
fi

# Set the project
gcloud config set project "${PROJECT_ID}"

# =============================================================================
# Enable required APIs
# =============================================================================
# echo "🔧 Enabling required APIs..."
# gcloud services enable \
#     cloudbuild.googleapis.com \
#     run.googleapis.com \
#     eventarc.googleapis.com \
#     artifactregistry.googleapis.com \
#     storage.googleapis.com

# =============================================================================
# Create Artifact Registry repository (if needed)
# =============================================================================
# echo "📦 Setting up Artifact Registry..."
# if ! gcloud artifacts repositories describe "${REPO_NAME}" \
#     --location="${REGION}" &>/dev/null; then
#     gcloud artifacts repositories create "${REPO_NAME}" \
#         --repository-format=docker \
#         --location="${REGION}" \
#         --description="Cloud Run function images"
# fi
echo "📦 Artifact Registry repo already exists, skipping..."
# Configure Docker authentication
# gcloud auth configure-docker "${REGION}-docker.pkg.dev" --quiet

# =============================================================================
# Build and push the container image
# =============================================================================
echo "🔨 Building container image..."

# Option 1: Build locally and push (faster iteration)
# docker build -t "${IMAGE_URI}" .
# docker push "${IMAGE_URI}"

# Option 2: Build with Cloud Build (no local Docker needed)
# Uncomment below and comment out the docker commands above to use Cloud Build
gcloud builds submit --tag "${IMAGE_URI}" .

# =============================================================================
# Deploy to Cloud Run
# =============================================================================
echo "🚀 Deploying to Cloud Run..."
gcloud run deploy "${SERVICE_NAME}" \
    --image="${IMAGE_URI}" \
    --region="${REGION}" \
    --platform=managed \
    --no-allow-unauthenticated \
    --memory=2Gi \
    --cpu=2 \
    --min-instances=0 \
    --max-instances=2 \
    --timeout=60s \
    --concurrency=1 # One image at a time, rayon uses both CPUs

# =============================================================================
# Set up Eventarc trigger for GCS events
# =============================================================================
echo "⚡ Creating Eventarc trigger..."

# Get the Cloud Storage service account for the project
GCS_SERVICE_ACCOUNT="$(gsutil kms serviceaccount -p "${PROJECT_ID}")"

# Grant the pubsub.publisher role to the GCS service account
# (required for Eventarc to receive GCS events)
echo "   Granting Pub/Sub publisher role to GCS service account..."
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
    --member="serviceAccount:${GCS_SERVICE_ACCOUNT}" \
    --role="roles/pubsub.publisher" \
    --condition=None \
    --quiet

# Get the default compute service account
COMPUTE_SA="$(gcloud projects describe "${PROJECT_ID}" --format='value(projectNumber)')-compute@developer.gserviceaccount.com"

# Grant eventarc.eventReceiver role to the compute service account
echo "   Granting Eventarc receiver role to compute service account..."
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
    --member="serviceAccount:${COMPUTE_SA}" \
    --role="roles/eventarc.eventReceiver" \
    --condition=None \
    --quiet

# Delete existing trigger if it exists (for re-deployment)
if gcloud eventarc triggers describe "${SERVICE_NAME}-trigger" \
    --location="${REGION}" &>/dev/null; then
    echo "   Deleting existing trigger..."
    gcloud eventarc triggers delete "${SERVICE_NAME}-trigger" \
        --location="${REGION}" \
        --quiet
fi

# Create the Eventarc trigger
# This triggers on object finalization (upload complete) in the specified bucket
echo "   Creating new trigger..."
gcloud eventarc triggers create "${SERVICE_NAME}-trigger" \
    --location="${REGION}" \
    --destination-run-service="${SERVICE_NAME}" \
    --destination-run-region="${REGION}" \
    --event-filters="type=google.cloud.storage.object.v1.finalized" \
    --event-filters="bucket=${BUCKET_NAME}" \
    --service-account="${COMPUTE_SA}"

# =============================================================================
# Set up lifecycle rules for buckets
# =============================================================================
echo "🗑️  Setting up lifecycle rules..."

# Create temporary lifecycle config
cat > /tmp/lifecycle-upload.json << 'EOF'
{
  "lifecycle": {
    "rule": [
      {
        "action": { "type": "Delete" },
        "condition": { "age": 1 }
      }
    ]
  }
}
EOF

# Apply to upload bucket (delete originals after 1 day)
gsutil lifecycle set /tmp/lifecycle-upload.json "gs://${BUCKET_NAME}"
echo "   Applied lifecycle rule to ${BUCKET_NAME}: delete after 1 day"

# Cleanup
rm /tmp/lifecycle-upload.json
# =============================================================================
# Done!
# =============================================================================
echo ""
echo "✅ Deployment complete!"
echo ""
echo "📊 To view logs:"
echo "   gcloud run logs read ${SERVICE_NAME} --region=${REGION} --limit=50"
echo ""
echo "🧪 To test, upload a file to your bucket:"
echo "   echo 'test' | gsutil cp - gs://${BUCKET_NAME}/test-file.txt"
echo ""
echo "🔍 To check trigger status:"
echo "   gcloud eventarc triggers describe ${SERVICE_NAME}-trigger --location=${REGION}"
