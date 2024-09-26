use std::sync::Arc;

use aws_lambda_events::sqs::SqsEvent;

use aws_sdk_dynamodb as dynamodb;
use dynamodb::error::BoxError;
use lambda_runtime::{run, service_fn, tracing, Error, LambdaEvent};

use producer::fetch_pricing;

use reqwest::Client;
use service::has_new_results;
use tokio::sync::mpsc;
use tracing::error;

use types::WorkerError;
use wh_core::time_provider;
use wh_core::types::BiddingZone;

mod consumer;
mod producer;
mod service;
mod types;

async fn handle_store_electricity_pricing(
    _event: LambdaEvent<SqsEvent>,
    dynamo_client: Arc<dynamodb::Client>,
    reqwest_client: Arc<reqwest::Client>,
) -> Result<(), BoxError> {
    let data = match fetch_pricing(&reqwest_client, &BiddingZone::FI).await {
        Ok(data) => data,
        Err(e) => return Err(Box::new(e)),
    };

    let new_pricing_data_available = has_new_results(data, &time_provider::SystemTimeProvider)?;

    if !new_pricing_data_available {
        error!("New electricity pricing data not available");
        return Err(Box::new(WorkerError::Data(
            "New electricity pricing data not available".to_string(),
        )));
    }

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
    tracing_subscriber::fmt()
        .json()
        .with_max_level(tracing::Level::INFO)
        .with_current_span(false)
        .with_ansi(false)
        .without_time()
        .with_target(false)
        .init();

    let config = aws_config::load_from_env().await;
    let dynamo_client = Arc::new(dynamodb::Client::new(&config));
    let reqwest_client = Arc::new(Client::new());

    run(service_fn(|event| {
        handle_store_electricity_pricing(event, dynamo_client.clone(), reqwest_client.clone())
    }))
    .await
}
