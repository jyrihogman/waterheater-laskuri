use aws_lambda_events::sqs::SqsEvent;

use aws_sdk_dynamodb as dynamodb;
use dynamodb::error::BoxError;
use lambda_runtime::{run, service_fn, tracing, Error, LambdaEvent};

use producer::fetch_pricing;

use reqwest::Client;
use service::has_new_results;
use tokio::sync::mpsc;
use types::WorkerError;
use wh_core::types::BiddingZone;

mod consumer;
mod producer;
mod service;
mod time_provider;
mod types;

async fn handle_store_electricity_pricing(_event: LambdaEvent<SqsEvent>) -> Result<(), BoxError> {
    let config = aws_config::load_from_env().await;
    let dynamo_client = dynamodb::Client::new(&config);
    let reqwest_client = Client::new();
    println!("{:?}", _event);

    let data = match fetch_pricing(&reqwest_client, &BiddingZone::FI).await {
        Ok(data) => data,
        Err(e) => return Err(Box::new(e)),
    };

    let new_pricing_data_available = has_new_results(data, &time_provider::SystemTimeProvider)?;

    if !new_pricing_data_available {
        eprintln!("New electricity pricing data not available");
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
    tracing::init_default_subscriber();

    run(service_fn(handle_store_electricity_pricing)).await
}
