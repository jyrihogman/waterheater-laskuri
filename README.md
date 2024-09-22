Service for calculating the cheapest x hour period for electricity. API is designed to be compatible with existing [Shelly scripts](https://spot-hinta.fi/shelly-skriptien-esittely/).

## Overview

- Axum HTTP server running in a Lambda behind an API Gateway serves a single endpoint which returns 200 if your device should be turned **on**, and 400 if **off**.
- Worker lambda which fetches electricity pricing for the current period and stores it
- Message handler lambda which handles backoff for retrying if new pricing data is not available
- Pulumi is the chosen IaC framework for the project

## Deployment

Changes are automatically deployed by the CI/CD pipeline for `main` branch.

Deployment can be done locally as well with `pulumi`, `docker` & `aws-cli`.

Infra related to the worker & message handler is located in [lambda-infra](lambda-infra/index.ts) directory. Before deployment, you need to set the `alarmEmail` configuration value. Both Lambdas are containerized and the containers are built & pushed to ECR separately in the CI/CD pipeline.

```bash
pulumi config set alarmEmail your_email@domain.com
docker build -t $REGISTRY/$REGISTRY_ALIAS/$REPOSITORY:latest -f worker/Dockerfile .
docker build -t $REGISTRY/$REGISTRY_ALIAS/$REPOSITORY:latest -f worker/message-handler .

docker push $REGISTRY/$REGISTRY_ALIAS/$REPOSITORY:latest

pulumi up
```

The **server** runs in a lambda (as well), but it's deployed as an asset. The package will be built on deploy.

```bash
pulumi up --yes
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

Running the server with live-reload can be done with cargo lambda

```bash
cargo lambda watch

# check api-docs
http://localhost:9000/lambda-url/waterheater-calc/api/v2/swagger-ui/

# query an endpoint
curl "http://localhost:9000/lambda-url/waterheater-calc/api/v2/waterheater/country/fi/cheapest-period?hours=1&start=0&end=5"
```
