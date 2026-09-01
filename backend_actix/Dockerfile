# NOTE: the build context is the REPOSITORY ROOT, not this directory.
# backend_actix is a member of the root Cargo workspace, so the build needs
# the root Cargo.toml/Cargo.lock and every member manifest to resolve.
#
#   docker build -f backend_actix/Dockerfile -t backend-actix .
#
# ./build.sh does this for you via cloudbuild.yaml.

# ---- build stage ----
FROM rust:1.88 as builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY backend_actix ./backend_actix
COPY image-processor-function ./image-processor-function

ENV RUSTFLAGS="-C target-cpu=x86-64-v3"
# --locked: fail loudly rather than silently re-resolving if the committed
# lockfile is out of date with the manifests.
RUN cargo build --release --locked -p backend_actix

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
