# Build stage
FROM rust:1.88 as builder

WORKDIR /app

# Install build dependencies (including jemalloc)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libjemalloc-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first for better layer caching
COPY Cargo.toml Cargo.lock* ./

# Create dummy src to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy actual source and rebuild
COPY src ./src
RUN touch src/main.rs && cargo build --release

# Runtime stage - minimal image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libjemalloc2 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/image-processor-function /app/image-processor-function

# Cloud Run runs as non-root by default, but let's be explicit
RUN useradd -r -u 1001 appuser
USER appuser

# Cloud Run sets PORT env var, default to 8080
ENV PORT=8080

EXPOSE 8080

CMD ["/app/image-processor-function"]
