ARG RUST_VERSION=1.80.0

FROM rust:${RUST_VERSION}-slim-bullseye AS build
WORKDIR /app

RUN apt-get update
RUN apt-get install -y pkg-config openssl libssl-dev curl

COPY ./server/ ./server/
COPY ./worker/ ./worker/
COPY ./wh-core/ ./wh-core/
COPY Cargo.toml .
COPY Cargo.lock .
RUN cd server && cargo build -p waterheater-calc --release

FROM debian:bullseye-slim AS final

RUN apt-get update && apt-get install -y ca-certificates pkg-config openssl libssl-dev curl && update-ca-certificates
WORKDIR /app/server
COPY --from=build /app/target/release/waterheater-calc .

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
