FROM ghcr.io/cargo-lambda/cargo-lambda as builder

RUN apt-get update
RUN apt-get install musl-tools pkg-config libssl-dev -y

# Create a new empty shell project
WORKDIR /usr/src/app

COPY Cargo.lock .
COPY Cargo.toml .
COPY ./message-handler/ ./message-handler
COPY ./server/ ./server/
COPY ./worker/ ./worker/
COPY ./wh-core/ ./wh-core/

# Build the release
RUN cargo lambda build -p waterheater-calc --release

# # Copy the built executable to the Lambda base image
FROM public.ecr.aws/lambda/provided:al2023
COPY --from=builder /usr/src/app/target/lambda/waterheater-calc/bootstrap /var/runtime/

CMD [ "bootstrap" ]


# FROM rust:1.81-slim-bullseye AS builder
# WORKDIR /app
#
# RUN apt-get update
# RUN apt-get install -y pkg-config openssl libssl-dev curl
#
# COPY ./server/ ./server/
# COPY ./worker/ ./worker/
# COPY ./wh-core/ ./wh-core/
# COPY ./message-handler/ ./message-handler/
# COPY Cargo.toml .
# COPY Cargo.lock .
#
# WORKDIR /app/server
# RUN cargo build -p waterheater-calc --release
#
# FROM debian:bullseye-slim AS final
# WORKDIR /app/server
# COPY --from=builder /app/target/release/waterheater-calc .
#
# ARG UID=10001
# RUN adduser \
#   --disabled-password \
#   --gecos "" \
#   --home "/nonexistent" \
#   --shell "/sbin/nologin" \
#   --no-create-home \
#   --uid "${UID}" \
#   appuser
# RUN chown -R appuser:appuser /app && chmod -R 755 /app
# USER appuser
#
# CMD ["/app/server/server"]
# EXPOSE 8001
