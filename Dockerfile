# syntax=docker/dockerfile:1
# DHUN Global Shared Song Metadata Catalogue Service (`dhun-server`)
# Stores ONLY reusable song metadata (engine-only HMAC auth; zero parties, zero audio files).

# 1. Build Stage
FROM rust:1.80-slim-bullseye AS builder

WORKDIR /usr/src/dhun

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libasound2-dev \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml ./
COPY src-tauri ./src-tauri

RUN cargo build --release --bin dhun-server

# 2. Runtime Stage
FROM debian:bullseye-slim AS runner

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libasound2 \
    sqlite3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/dhun/target/release/dhun-server /usr/local/bin/dhun-server

RUN mkdir -p /app/data && chmod 777 /app/data

ENV PORT=8080
ENV DATABASE_URL=/app/data/dhun_global_catalogue.db

EXPOSE 8080

CMD ["dhun-server"]
