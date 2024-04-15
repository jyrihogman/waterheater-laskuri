use aws_lambda_events::event::cloudwatch_events::CloudWatchEvent;
use lambda_runtime::{run, service_fn, tracing, Error, LambdaEvent};

use service::{fetch_pricing_data, process_and_store_data, WorkerError};

mod service;

async fn handle_store_electricity_pricing(
    _event: LambdaEvent<CloudWatchEvent>,
) -> Result<(), WorkerError> {
    let pricing_data = fetch_pricing_data().await?;
    process_and_store_data(pricing_data).await
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    run(service_fn(handle_store_electricity_pricing)).await
}
