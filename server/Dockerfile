FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 
COPY --from=planner /app/recipe.json recipe.json
RUN apt-get update
RUN apt-get install -y pkg-config openssl libssl-dev curl
COPY ./server/ ./server/
COPY ./worker/ ./worker/
COPY ./wh-core/ ./wh-core/
COPY Cargo.toml .
COPY Cargo.lock .
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
WORKDIR /app/server
RUN cargo build -p waterheater-calc --release

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y ca-certificates pkg-config openssl libssl-dev curl && update-ca-certificates
WORKDIR /app/server
COPY --from=builder /app/target/release/waterheater-calc .

ARG UID=10001
RUN adduser \
  --disabled-password \
  --gecos "" \
  --home "/nonexistent" \
  --shell "/sbin/nologin" \
  --no-create-home \
  --uid "${UID}" \
  appuser
RUN chown -R appuser:appuser /app && chmod -R 755 /app
USER appuser

CMD ["/app/server/server"]
EXPOSE 8001
