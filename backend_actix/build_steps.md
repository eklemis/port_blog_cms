# Cloud Run Deployment Scripts

Two-script approach for faster iteration:

## Scripts

### `build.sh` - Build Container Image
Run this **only when your code changes** (~15-20 minutes)

```bash
chmod +x build.sh
./build.sh
```

**What it does:**
- Builds your Rust application using Cloud Build
- Pushes image to Google Container Registry
- Uses faster build machines (e2-highcpu-8)

**Environment variables:**
- `SERVICE_NAME` - Service name (default: backend-actix)
- `PROJECT_ID` - GCP project (auto-detected from gcloud config)
- `IMAGE_TAG` - Image tag (default: latest)

### `deploy.sh` - Deploy to Cloud Run
Run this **every time you need to update configuration** (~1-2 minutes)

```bash
chmod +x deploy.sh
./deploy.sh
```

**What it does:**
- Creates/updates secrets in Secret Manager
- Prompts for configuration (CPU, memory, env vars)
- Grants permissions to service account
- **Applies pending database migrations**
- Deploys the pre-built image to Cloud Run

### Migrations

Migrations run *before* the Cloud Run update, and a failure aborts the deploy.
The container does not migrate on startup.

The reasoning is in [ADR 0003](docs/adr/0003-migrate-before-deploy.md), and the
constraint it puts on migrations is refined by
[ADR 0010](docs/adr/0010-migrations-must-be-backward-compatible.md): a migration
must be backward-compatible with the build still running when it applies, which
is a stricter test than "additive". Read both before writing a migration that
drops or renames anything — or that adds a constraint, which is additive and
still not safe on its own.

`deploy.sh` shows `migration status` and asks for confirmation before applying.

| Variable | Effect |
| --- | --- |
| `SKIP_MIGRATIONS=1` | Skip the step entirely |
| `AUTO_MIGRATE=1` | Apply without the confirmation prompt (for CI) |

Running migrations needs `cargo` on the deploying machine. If it is missing the
script stops rather than silently skipping, since a quiet skip is exactly the
failure this step exists to prevent.

**Environment variables:**
- All the same as build.sh, plus:
- `REGION` - Cloud Run region (auto-detected from gcloud config)
- Any secret/config values (DATABASE_URL, REDIS_URL, etc.)

## Typical Workflow

### First time setup:
```bash
# 1. Build the image
./build.sh

# 2. Deploy with configuration
./deploy.sh
```

### When you change code:
```bash
# 1. Build new image
./build.sh

# 2. Deploy (reuses existing config if you set env vars)
export DATABASE_URL="postgresql://..."
export REDIS_URL="rediss://..."
# ... set other secrets
./deploy.sh
```

### When you only change configuration (fast!):
```bash
# Just deploy with new config
./deploy.sh
```

## Using Different Image Tags

To deploy a specific version:

```bash
# Build with a tag
IMAGE_TAG="v1.2.3" ./build.sh

# Deploy that specific tag
IMAGE_TAG="v1.2.3" ./deploy.sh
```

Or use git commit hash:

```bash
# Build
IMAGE_TAG="$(git rev-parse --short HEAD)" ./build.sh

# Deploy
IMAGE_TAG="$(git rev-parse --short HEAD)" ./deploy.sh
```

## Skip Prompts with Environment Variables

Set all values as environment variables to skip prompts:

```bash
export DATABASE_URL="postgresql://..."
export REDIS_URL="rediss://..."
export JWT_SECRET="your-secret"
export SMTP_PASSWORD="smtp-pass"
export RUST_ENV="production"
export CPU="2"
export MEMORY="1Gi"
export MULTIMEDIA_UPLOAD_BUCKET="your-bucket-name"
export MEDIA_STALE_UPLOAD_SECS="3600"
export AUTO_MIGRATE="1"          # skip the migration confirmation too
# ... etc

./deploy.sh  # No prompts!
```

`MAINTENANCE_TOKEN` is deliberately absent from that list. It is machine-only —
Cloud Scheduler sends it, the service compares it — so `deploy.sh` generates one
on the first deploy and then leaves it alone. Re-generating it on later deploys
would rotate it out from under the scheduler job, whose header still carries the
old value; the sweep would answer 404 and go on answering it, which looks exactly
like a sweep that ran and found nothing.

## Time Savings

- **Full deployment (build + deploy)**: 15-20 minutes
- **Config-only deployment**: 1-2 minutes
- **Savings**: 90% faster when you only need to update configuration!
