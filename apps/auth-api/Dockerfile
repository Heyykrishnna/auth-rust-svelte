# syntax=docker/dockerfile:1

# ─── Stage 1: Builder ──────────────────────────────────────────────────────────
FROM rust:1.80-alpine AS builder

# Install build dependencies
RUN apk add --no-cache \
    musl-dev \
    pkgconfig \
    openssl-dev \
    openssl-libs-static \
    protobuf-dev

WORKDIR /app

# Cache dependencies separately
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release --locked 2>/dev/null || true
RUN rm -f target/release/deps/auth_api*

# Copy source and build
COPY . .
RUN cargo build --release --locked

# ─── Stage 2: Runtime (distroless) ─────────────────────────────────────────────
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/auth-api /app/auth-api

# Copy migrations
COPY --from=builder /app/migrations /app/migrations

EXPOSE 8080

ENV AUTH_API_HOST=0.0.0.0
ENV AUTH_API_PORT=8080

ENTRYPOINT ["/app/auth-api"]
