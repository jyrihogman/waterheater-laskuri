use std::sync::Arc;

use aws_sdk_dynamodb as dynamodb;
use dynamodb::{operation::put_item::PutItemError, types::AttributeValue};

use chrono::{offset::LocalResult, DateTime, Days, TimeZone};
use chrono_tz::Tz;
use futures::future::try_join_all;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use thiserror::Error;
use tokio::task::{JoinError, JoinSet};
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

pub async fn fetch_pricing_data() -> Result<Vec<(BiddingZone, EnergyChartApiResponse)>, WorkerError>
{
    let mut set = JoinSet::new();
    let client = reqwest::Client::new();

    for zone in BiddingZone::iter() {
        let cloned_client = client.clone();
        set.spawn(async move {
            get_electricity_pricing(&cloned_client, &zone)
                .await
                .map(|data| (zone, data))
        });
    }

    let mut results: Vec<(BiddingZone, EnergyChartApiResponse)> = vec![];

    while let Some(res) = set.join_next().await {
        let out = res??;
        results.push(out);
    }

    Ok(results)
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

pub async fn process_and_store_data(
    data: Vec<(BiddingZone, EnergyChartApiResponse)>,
) -> Result<(), WorkerError> {
    let config = aws_config::load_from_env().await;
    let client = dynamodb::Client::new(&config);

    let futures: Vec<_> = data
        .into_iter()
        .map(|(zone, pricing_data)| {
            let cloned_client = client.clone();
            tokio::spawn(async move {
                let parsed_data = parse_pricing_data(&zone.to_tz(), &pricing_data)?;
                store_pricing_data(cloned_client, &zone, &parsed_data).await
            })
        })
        .collect();

    let results = try_join_all(futures).await?;

    for result in results {
        match result {
            Ok(_) => println!("Data stored successfully"),
            Err(e) => println!("Error storing data: {:?}", e),
        }
    }

    Ok(())
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
            println!("Pricing successfully inserted to DynamoDB");
        })
        .map_err(|e| Box::new(e.into_service_error()))?;

    Ok(())
}
