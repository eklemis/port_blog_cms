# GCS Upload Function - Rust / Actix Web

A Cloud Run function that processes Google Cloud Storage upload events using Actix Web.

## Architecture

```
┌─────────────┐     ┌───────────┐     ┌─────────────────┐
│ GCS Bucket  │────▶│ Eventarc  │────▶│ Cloud Run       │
│ (upload)    │     │ (trigger) │     │ (this function) │
└─────────────┘     └───────────┘     └─────────────────┘
```

When a file is uploaded to the configured GCS bucket:
1. GCS emits a `google.cloud.storage.object.v1.finalized` event
2. Eventarc routes the CloudEvent to your Cloud Run service
3. Your function processes the event (CloudEvent headers + JSON body)

## Quick Start

### Prerequisites

- Google Cloud project with billing enabled
- `gcloud` CLI installed and authenticated
- Docker (for local builds) or Cloud Build enabled
- A GCS bucket in the same project

### Deploy

```bash
# Set your configuration
export PROJECT_ID=your-project-id
export BUCKET_NAME=your-bucket-name
export REGION=us-central1  # optional, defaults to us-central1

# Make deploy script executable and run
chmod +x deploy.sh
./deploy.sh
```

### Test the Deployment

```bash
# Upload a test file
echo "hello world" | gsutil cp - gs://your-bucket-name/test.txt

# Check the logs
gcloud run logs read gcs-upload-function --region=us-central1 --limit=20
```

## Local Development

```bash
# Run the server
cargo run

# In another terminal, simulate a GCS event
chmod +x test-local.sh
./test-local.sh
```

## Project Structure

```
.
├── Cargo.toml          # Dependencies
├── src/
│   └── main.rs         # Application code
├── Dockerfile          # Multi-stage build for Cloud Run
├── deploy.sh           # One-click deployment script
├── test-local.sh       # Local testing helper
└── README.md
```

## CloudEvent Format

Cloud Run receives GCS events in CloudEvent "binary" format:

**Headers:**
- `ce-id`: Unique event ID
- `ce-source`: `//storage.googleapis.com/projects/_/buckets/{bucket}`
- `ce-type`: `google.cloud.storage.object.v1.finalized`
- `ce-subject`: `objects/{object-path}`

**Body (JSON):**
```json
{
  "bucket": "my-bucket",
  "name": "path/to/file.jpg",
  "contentType": "image/jpeg",
  "size": "12345",
  "timeCreated": "2024-01-15T10:30:00.000Z",
  ...
}
```

## Customization

### Handle Different Event Types

The function currently only processes `finalized` events. To handle deletions, archives, etc.:

```rust
match headers.event_type.as_str() {
    "google.cloud.storage.object.v1.finalized" => { /* upload */ }
    "google.cloud.storage.object.v1.deleted" => { /* deletion */ }
    "google.cloud.storage.object.v1.archived" => { /* archive */ }
    _ => { /* unknown */ }
}
```

### Filter by Path Prefix

Add path filtering in the Eventarc trigger:

```bash
gcloud eventarc triggers create my-trigger \
    --event-filters="type=google.cloud.storage.object.v1.finalized" \
    --event-filters="bucket=my-bucket" \
    --event-filters-path-pattern="name=/uploads/*" \  # Only /uploads/ prefix
    ...
```

### Add GCS Client for File Processing

Add to `Cargo.toml`:
```toml
google-cloud-storage = "0.15"
```

Then in your handler:
```rust
use google_cloud_storage::client::{Client, ClientConfig};

let config = ClientConfig::default().with_auth().await?;
let client = Client::new(config);

// Download the uploaded file
let data = client
    .download_object(&gcs_data.bucket, &gcs_data.name)
    .await?;
```

## Troubleshooting

### Events not arriving

1. Check the trigger exists:
   ```bash
   gcloud eventarc triggers list --location=us-central1
   ```

2. Verify IAM permissions:
   ```bash
   # GCS needs pubsub.publisher
   gsutil kms serviceaccount -p PROJECT_ID
   ```

3. Check Eventarc audit logs in Cloud Console

### Function errors

```bash
# View recent logs
gcloud run logs read gcs-upload-function --region=us-central1 --limit=50

# Stream logs
gcloud run logs tail gcs-upload-function --region=us-central1
```

## Cost Optimization

- `min-instances=0`: Scale to zero when idle (default)
- `memory=256Mi`: Minimal memory for simple processing
- `timeout=60s`: Fast timeout for event processing
- Consider using Cloud Run jobs for batch processing
