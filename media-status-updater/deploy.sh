#!/bin/bash

# --- CONFIGURATION ---
PROJECT_ID=$(gcloud config get-value project)
PROJECT_NUMBER=$(gcloud projects describe $PROJECT_ID --format='value(projectNumber)')
FUNCTION_NAME="media-status-updater"
REGION="asia-southeast2"
RUNTIME="nodejs22" # Use the latest stable Node runtime
ENTRY_POINT="updateMediaRecord" # The name of the function in your index.js
BUCKET_NAME="blogport-cms-manifests"

# Service Account Details
SA_NAME="manifest-processor-sa"
SA_EMAIL="$SA_NAME@$PROJECT_ID.iam.gserviceaccount.com"

echo "🛠️  Starting Infrastructure Setup..."

# 1. Create Service Account (if not exists)
if ! gcloud iam service-accounts describe $SA_EMAIL >/dev/null 2>&1; then
  echo "👤 Creating Service Account: $SA_EMAIL"
  gcloud iam service-accounts create $SA_NAME --display-name="Manifest Processor Identity"
else
  echo "✅ Service Account already exists."
fi

# 2. Assign Necessary Roles to the Service Account
echo "🔐 Assigning roles to $SA_EMAIL..."
ROLES=(
  "roles/logging.logWriter"           # To write logs
  "roles/storage.objectViewer"        # To read manifest.json
  "roles/eventarc.eventReceiver"      # To receive GCS events
  "roles/secretmanager.secretAccessor" # To read DB password
  "roles/run.invoker"                 # Required for Eventarc to call the function
)

for role in "${ROLES[@]}"; do
  gcloud projects add-iam-policy-binding $PROJECT_ID \
    --member="serviceAccount:$SA_EMAIL" \
    --role="$role" --quiet >/dev/null
done

# 3. CRITICAL: Grant GCS Service Agent permission to publish events
# Without this, the GCS trigger will never fire!
GCS_SERVICE_ACCOUNT="service-$PROJECT_NUMBER@gs-project-accounts.iam.gserviceaccount.com"
echo "📡 Granting Pub/Sub permission to GCS Agent: $GCS_SERVICE_ACCOUNT"
gcloud projects add-iam-policy-binding $PROJECT_ID \
    --member="serviceAccount:$GCS_SERVICE_ACCOUNT" \
    --role="roles/pubsub.publisher" --quiet >/dev/null

# --- DEPLOYMENT ---
# This command creates/updates the function AND the Eventarc trigger automatically
echo "🚀 Deploying $FUNCTION_NAME to $REGION..."

# 5. Deploy the Function
echo "🚀 Deploying $FUNCTION_NAME..."
gcloud functions deploy $FUNCTION_NAME \
  --gen2 \
  --runtime=nodejs22 \
  --region=$REGION \
  --entry-point=$ENTRY_POINT \
  --source=. \
  --service-account=$SA_EMAIL \
  --trigger-bucket=$BUCKET_NAME \
  --trigger-service-account=$SA_EMAIL \
  --max-instances=2

echo "🎉 All done! Your function is now running under its own identity."
