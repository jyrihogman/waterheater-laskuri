FROM ghcr.io/cargo-lambda/cargo-lambda as builder

RUN apt-get update
RUN apt-get install musl-tools -y

# Create a new empty shell project
WORKDIR /usr/src/app

COPY Cargo.lock .
COPY Cargo.toml .
COPY ./message-handler/ ./message-handler
COPY ./server/ ./server/
COPY ./worker/ ./worker/
COPY ./wh-core/ ./wh-core/

# Build the release
RUN cargo lambda build -p worker --release

# # Copy the built executable to the Lambda base image
FROM public.ecr.aws/lambda/provided:al2023
COPY --from=builder /usr/src/app/target/lambda/worker/bootstrap /var/runtime/
CMD [ "bootstrap" ]
