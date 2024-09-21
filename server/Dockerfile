FROM rust:1.81-slim-bullseye AS builder
WORKDIR /app

RUN apt-get update
RUN apt-get install -y pkg-config openssl libssl-dev curl

COPY ./server/ ./server/
COPY ./worker/ ./worker/
COPY ./wh-core/ ./wh-core/
COPY ./message-handler/ ./message-handler/
COPY Cargo.toml .
COPY Cargo.lock .

WORKDIR /app/server
RUN cargo build -p waterheater-calc --release

FROM debian:bullseye-slim AS final
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
