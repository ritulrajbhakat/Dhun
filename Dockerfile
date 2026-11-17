# syntax=docker/dockerfile:1
# DHUN Cloud Server — Production Multi-Stage Containerfile
# Compatible with Render, Railway, Fly.io, and Docker Compose

# 1. Build Stage
FROM rust:1.80-slim-bullseye AS builder

WORKDIR /usr/src/dhun

# Install system dependencies required for building audio, TLS, and Tauri build scripts
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libasound2-dev \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy Workspace manifest and backend source
COPY Cargo.toml ./
COPY src-tauri ./src-tauri

# Compile the headless cloud server binary in release mode
RUN cargo build --release --bin dhun-server

# 2. Runtime Stage
FROM debian:bullseye-slim AS runner

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libasound2 \
    sqlite3 \
    && rm -rf /var/lib/apt/lists/*

# Copy compiled cloud server executable from workspace target directory
COPY --from=builder /usr/src/dhun/target/release/dhun-server /usr/local/bin/dhun-server

# Create data directory for SQLite persistent database volume
RUN mkdir -p /app/data && chmod 777 /app/data

ENV PORT=8080
ENV DATABASE_URL=/app/data/dhun.db
ENV PUBLIC_GUEST_URL=https://ritulrajbhakat.github.io/Dhun
ENV CORS_ORIGIN=*

EXPOSE 8080

CMD ["dhun-server"]
