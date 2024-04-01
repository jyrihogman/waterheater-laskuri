ARG RUST_VERSION=1.76.0

FROM rust:${RUST_VERSION}-slim-bullseye AS build
WORKDIR /app

RUN apt-get update
RUN apt-get install -y pkg-config openssl libssl-dev

COPY . .

RUN cargo build --release

FROM debian:bullseye-slim AS final

RUN apt-get update && apt-get install -y ca-certificates pkg-config openssl libssl-dev curl && update-ca-certificates
WORKDIR /app
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

CMD ["/app/waterheater-calc"]

EXPOSE 8001
