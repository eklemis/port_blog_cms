# ---- build stage ----
FROM rust:1.88 as builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY . .

ENV RUSTFLAGS="-C target-cpu=x86-64-v3"
RUN cargo build --release

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
