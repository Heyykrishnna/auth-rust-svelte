FROM rust:1.96.0-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --locked --release --bins && \
    mkdir /app/bin && \
    cp target/release/auth-api target/release/webhook-worker /app/bin/

FROM builder AS development

RUN cargo install cargo-watch --version 8.5.3 --locked

FROM alpine:3.24 AS runtime

RUN apk upgrade --no-cache && apk add --no-cache ca-certificates

WORKDIR /app

COPY --from=builder /app/bin/auth-api /app/auth-api
COPY --from=builder /app/bin/webhook-worker /app/webhook-worker
COPY --from=builder /app/migrations /app/migrations

EXPOSE 8080

ENV HOST=0.0.0.0
ENV PORT=8080

USER 65534:65534

ENTRYPOINT ["/app/auth-api"]
