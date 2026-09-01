# NOTE: the build context is the REPOSITORY ROOT, not this directory.
# This crate is a member of the root Cargo workspace, so the build needs the
# root Cargo.toml/Cargo.lock and every member manifest to resolve.
#
#   docker build -f image-processor-function/Dockerfile -t image-processor .
#
# ./deploy.sh does this for you via cloudbuild.yaml.
#
# The old dummy-src dependency-caching trick was dropped in the workspace
# move: it cannot express four member crates cleanly, and Cloud Build starts
# from a cold layer cache anyway, so it was buying nothing here.

# Build stage
FROM rust:1.88 as builder
WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libjemalloc-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY image-processor-function ./image-processor-function
COPY backend_actix ./backend_actix

RUN cargo build --release --locked -p image-processor-function

# Runtime stage - minimal image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libjemalloc2 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/image-processor-function /app/image-processor-function

# Cloud Run runs as non-root by default, but let's be explicit
RUN useradd -r -u 1001 appuser
USER appuser

# Cloud Run sets PORT env var, default to 8080
ENV PORT=8080

EXPOSE 8080

CMD ["/app/image-processor-function"]
