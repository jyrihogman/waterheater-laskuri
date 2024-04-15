use std::sync::Arc;

use aws_sdk_dynamodb as dynamodb;
use dynamodb::{operation::put_item::PutItemError, types::AttributeValue};

use chrono::{offset::LocalResult, DateTime, Days, TimeZone};
use chrono_tz::Tz;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use thiserror::Error;
use tokio::{
    sync::mpsc,
    task::{JoinError, JoinHandle},
};
use url::form_urlencoded;

use wh_core::types::BiddingZone;

// Full example URL for the call
// https://api.energy-charts.info/price?bzn=FI&start=2024-06-01T00%3A00%2B01%3A00&end=2024-06-01T23%3A45%2B01%3A00
static BASE_URL: &str = "https://api.energy-charts.info/price";

#[derive(Error, Debug)]
pub enum WorkerError {
    #[error("Error from API: {0}")]
    Api(#[from] reqwest::Error),
    #[error("Error parsing data: {0}")]
    Parse(String),
    #[error("DynamoDB Put Item operation failed: {0}")]
    DatabaseError(#[from] Box<PutItemError>),
    #[error("Joining futures failed: {0}")]
    JoinError(#[from] JoinError),
}

#[derive(Debug, Deserialize)]
pub struct EnergyChartApiResponse {
    pub unix_seconds: Arc<[u32]>,
    pub price: Arc<[f32]>,
}

#[derive(Debug, Serialize)]
pub struct HourlyPrice(DateTime<Tz>, f32);

pub async fn fetch_pricing_data(
    client: &Client,
    tx: mpsc::Sender<(BiddingZone, EnergyChartApiResponse)>,
) -> Result<(), WorkerError> {
    let mut handles: Vec<JoinHandle<()>> = vec![];

    for zone in BiddingZone::iter() {
        let client = client.clone();
        let sender = tx.clone();

        let handle = tokio::spawn(async move {
            match get_electricity_pricing(&client, &zone).await {
                Ok(data) => {
                    if sender.send((zone, data)).await.is_err() {
                        eprintln!("Failed to send data through channel");
                    }
                }
                Err(e) => eprintln!("Failed to fetch data for zone {:?}: {:?}", zone, e),
            }
        });
        handles.push(handle);
    }

    // Waits for all tasks to complete
    for handle in handles {
        let _ = handle.await;
    }

    drop(tx); // Close the channel
    Ok(())
}

pub async fn process_and_store_data(
    client: &dynamodb::Client,
    mut receiver: mpsc::Receiver<(BiddingZone, EnergyChartApiResponse)>,
) -> Result<(), WorkerError> {
    while let Some((zone, data)) = receiver.recv().await {
        let cloned_client = client.clone();
        // Process each item as it arrives
        println!("Processing data for zone: {:?}", zone);
        let parsed_data = parse_pricing_data(&zone.to_tz(), &data)?;
        store_pricing_data(cloned_client, &zone, &parsed_data).await?;
    }
    Ok(())
}

pub async fn get_electricity_pricing(
    client: &Client,
    timezone: &BiddingZone,
) -> Result<EnergyChartApiResponse, reqwest::Error> {
    let start_date = timezone
        .to_tz()
        .from_utc_datetime(&chrono::Utc::now().naive_utc());
    let end_date = match start_date.checked_add_days(Days::new(1)) {
        Some(d) => d,
        None => start_date,
    };

    let url: String = format!(
        "{}?bzn={}&start={}&end={}",
        BASE_URL,
        timezone,
        form_urlencoded::byte_serialize(start_date.to_rfc3339().as_bytes()).collect::<String>(),
        form_urlencoded::byte_serialize(end_date.to_rfc3339().as_bytes()).collect::<String>()
    );

    client
        .get(url)
        .send()
        .await
        .map_err(|e| {
            println!("Error: {}", e);
            e
        })?
        .json::<EnergyChartApiResponse>()
        .await
        .map_err(|e| {
            println!("Error: {}", e);
            e
        })
}

fn parse_pricing_data(
    timezone: &Tz,
    pricing_data: &EnergyChartApiResponse,
) -> Result<Vec<HourlyPrice>, WorkerError> {
    let mut kwh_pricing_data: Vec<HourlyPrice> = vec![];

    for (index, price) in pricing_data.price.iter().enumerate() {
        let unix_timestamp = pricing_data.unix_seconds[index] as i64;

        let date_time = match timezone.timestamp_opt(unix_timestamp, 0) {
            LocalResult::Single(d) => d,
            _ => {
                return Err(WorkerError::Parse(
                    "Formatting unix timestamp to datetime failed".to_string(),
                ))
            }
        };

        kwh_pricing_data.push(HourlyPrice(date_time, price / 1000_f32))
    }

    Ok(kwh_pricing_data)
}

async fn store_pricing_data(
    client: dynamodb::Client,
    bzn: &BiddingZone,
    pricing: &[HourlyPrice],
) -> Result<(), WorkerError> {
    client
        .put_item()
        .table_name("electricity_pricing_info")
        .item("PricingId", AttributeValue::S(format!("Pricing{}", bzn)))
        .item(
            "Data",
            AttributeValue::S(serde_json::to_string(pricing).unwrap()),
        )
        .send()
        .await
        .inspect(|_| {
            println!(
                "Pricing successfully inserted to DynamoDB for zone: {}",
                bzn
            );
        })
        .map_err(|e| Box::new(e.into_service_error()))?;

    Ok(())
}
