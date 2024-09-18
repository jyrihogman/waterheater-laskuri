Service for calculating the cheapest x hour period for electricity. API is designed to be compatible with existing [Shelly scripts](https://spot-hinta.fi/shelly-skriptien-esittely/).

## Overview

- Axum HTTP server which is currently hosted with AWS App Runner. Currently serves a single endpoint, which returns 200 if your device should be turned **on**, and 400 if **off**.
- Worker lambda which fetches electricity pricing for the current period and stores it
- Message handler lambda which handles backoff for retrying if new pricing data is not available
- Pulumi is the chosen IaC framework for the project

## Deployment

Changes are automatically deployed by the CI/CD pipeline for `main` branch.

Deployment can be done locally as well with `pulumi`, `docker` & `aws-cli`.

Lambda infra is located in [lambda-infra](lambda-infra/index.ts) directory. Before deployment, you need to set the `alarmEmail` configuration value.

```bash
pulumi config set alarmEmail your_email@domain.com
pulumi up
```

The **server** runs in a container in AWS App Runner. To deploy it locally, you need to build the image and push it to the repository of your choice, and initiate the deployment.

```bash
docker build -t $REGISTRY/$REGISTRY_ALIAS/$REPOSITORY:latest -f server/Dockerfile .
docker push $REGISTRY/$REGISTRY_ALIAS/$REPOSITORY:latest

aws apprunner start-deployment --service-arn ${{ secrets.AWS_APP_RUNNER_ARN }}
```

## Development

### Lambdas

Message handler & worker lambdas can be developed locally with `cargo lambda`.

```bash
brew tap cargo-lambda/cargo-lambda
brew install cargo-lambda

# in worker / message-handler directory
cargo lambda watch
```

To invoke the locally running lambda with an SQS Message

```bash
cargo lambda invoke worker --data-ascii "{ \"Records\": [] }"
cargo lambda invoke message-handler --data-ascii "{ \"Records\": [] }"
```

### Server

To run the server and have it live reload, you'll need `cargo-watch`

```bash
cargo install cargo-watch
cargo watch -x run
```
