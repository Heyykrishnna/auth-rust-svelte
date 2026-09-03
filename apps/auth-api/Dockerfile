FROM rust:1.80-alpine AS builder

RUN apk add --no-cache \
    musl-dev \
    pkgconfig \
    openssl-dev \
    openssl-libs-static \
    protobuf-dev

WORKDIR /app

COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release 2>/dev/null || true
RUN rm -f target/release/deps/auth_api*

COPY . .
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12:nonroot AS runtime

WORKDIR /app

COPY --from=builder /app/target/release/auth-api /app/auth-api
COPY --from=builder /app/migrations /app/migrations

EXPOSE 8080

ENV HOST=0.0.0.0
ENV PORT=8080

ENTRYPOINT ["/app/auth-api"]
