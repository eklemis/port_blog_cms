#!/bin/bash

# --- CONFIGURATION ---
PROJECT_ID=$(gcloud config get-value project)
PROJECT_NUMBER=$(gcloud projects describe $PROJECT_ID --format='value(projectNumber)')
FUNCTION_NAME="media-status-updater"
REGION="asia-southeast2"
RUNTIME="nodejs22"
ENTRY_POINT="updateMediaRecord"
BUCKET_NAME="blogport-cms-manifests"

# Service Account Details
SA_NAME="manifest-processor-sa"
SA_EMAIL="$SA_NAME@$PROJECT_ID.iam.gserviceaccount.com"

# Database Configuration
# Set this to your actual database connection string
# For Neon: postgresql://username:password@host.neon.tech/dbname?sslmode=require
DATABASE_URL="${DATABASE_URL:-}"

# --- VALIDATION ---
if [ -z "$DATABASE_URL" ]; then
  echo "❌ ERROR: DATABASE_URL environment variable is not set!"
  echo "Please set it before running this script:"
  echo "  export DATABASE_URL='postgresql://user:pass@host/db'"
  exit 1
fi

echo "🛠️  Starting Infrastructure Setup..."

# 1. Create Service Account (if not exists)
if ! gcloud iam service-accounts describe $SA_EMAIL >/dev/null 2>&1; then
  echo "👤 Creating Service Account: $SA_EMAIL"
  gcloud iam service-accounts create $SA_NAME \
    --display-name="Manifest Processor Identity"
else
  echo "✅ Service Account already exists."
fi

# 2. Assign Necessary Roles to the Service Account
echo "🔐 Assigning roles to $SA_EMAIL..."
ROLES=(
  "roles/logging.logWriter"           # To write logs
  "roles/storage.objectViewer"        # To read manifest.json
  "roles/eventarc.eventReceiver"      # To receive GCS events
  "roles/run.invoker"                 # Required for Eventarc to call the function
)

for role in "${ROLES[@]}"; do
  gcloud projects add-iam-policy-binding $PROJECT_ID \
    --member="serviceAccount:$SA_EMAIL" \
    --role="$role" --quiet >/dev/null
done
echo "✅ Roles assigned."

# 3. CRITICAL: Grant GCS Service Agent permission to publish events
# Without this, the GCS trigger will never fire!
GCS_SERVICE_ACCOUNT="service-$PROJECT_NUMBER@gs-project-accounts.iam.gserviceaccount.com"
echo "📡 Granting Pub/Sub permission to GCS Agent: $GCS_SERVICE_ACCOUNT"
gcloud projects add-iam-policy-binding $PROJECT_ID \
    --member="serviceAccount:$GCS_SERVICE_ACCOUNT" \
    --role="roles/pubsub.publisher" --quiet >/dev/null
echo "✅ Pub/Sub permission granted."

# Grant access to service account
gcloud secrets add-iam-policy-binding DATABASE_URL \
  --member="serviceAccount:$SA_EMAIL" \
  --role="roles/secretmanager.secretAccessor"
echo "✅ Secret access granted."

# --- DEPLOYMENT ---
echo ""
echo "🚀 Deploying $FUNCTION_NAME to $REGION..."
echo "   Bucket trigger: gs://$BUCKET_NAME"
echo "   Runtime: $RUNTIME"
echo "   Entry point: $ENTRY_POINT"
echo ""

gcloud functions deploy $FUNCTION_NAME \
  --gen2 \
  --runtime=$RUNTIME \
  --region=$REGION \
  --entry-point=$ENTRY_POINT \
  --source=. \
  --service-account=$SA_EMAIL \
  --trigger-bucket=$BUCKET_NAME \
  --trigger-service-account=$SA_EMAIL \
  --set-env-vars="DATABASE_URL=$DATABASE_URL,NODE_ENV=production" \
  --max-instances=5 \
  --min-instances=0 \
  --memory=256Mi \
  --timeout=60s \
  --quiet

if [ $? -eq 0 ]; then
  echo ""
  echo "🎉 Deployment successful!"
  echo ""
  echo "📊 Function details:"
  echo "   Name: $FUNCTION_NAME"
  echo "   Region: $REGION"
  echo "   Trigger: gs://$BUCKET_NAME (finalize events)"
  echo "   Service Account: $SA_EMAIL"
  echo ""
  echo "🧪 To test, upload a manifest.json to the bucket:"
  echo "   gsutil cp manifest.json gs://$BUCKET_NAME/test/manifest.json"
  echo ""
  echo "📋 View logs:"
  echo "   gcloud functions logs read $FUNCTION_NAME --region=$REGION --limit=50"
else
  echo ""
  echo "❌ Deployment failed. Check the error messages above."
  exit 1
fi
