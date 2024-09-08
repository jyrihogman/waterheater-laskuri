use aws_lambda_events::event::cloudwatch_events::CloudWatchEvent;
use aws_sdk_dynamodb as dynamodb;
use lambda_runtime::{run, service_fn, tracing, Error, LambdaEvent};

use reqwest::Client;
use tokio::sync::mpsc;

mod consumer;
mod producer;
mod types;

async fn handle_store_electricity_pricing(
    _event: LambdaEvent<CloudWatchEvent>,
) -> Result<(), Error> {
    let config = aws_config::load_from_env().await;
    let dynamo_client = dynamodb::Client::new(&config);
    let reqwest_client = Client::new();

    let (tx, rx) = mpsc::channel(32);

    let fetch_handle =
        tokio::spawn(async move { producer::get_pricing_data(&reqwest_client, tx).await });

    let process_handle =
        tokio::spawn(async move { consumer::process_and_store_data(&dynamo_client, rx).await });

    // Wait for both tasks to complete
    let _ = tokio::try_join!(fetch_handle, process_handle)?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    run(service_fn(handle_store_electricity_pricing)).await
}
