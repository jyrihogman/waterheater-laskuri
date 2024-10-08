use aws_sdk_dynamodb as dynamodb;
use dynamodb::types::AttributeValue;

use chrono::{offset::LocalResult, TimeZone};
use chrono_tz::Tz;
use tokio::sync::mpsc;
use tracing::{error, info};

use wh_core::types::BiddingZone;
use wh_core::util::get_storage_date;

use crate::types::{EnergyChartApiResponse, HourlyPrice, WorkerError};

pub async fn process_and_store_data(
    client: &dynamodb::Client,
    mut receiver: mpsc::Receiver<(BiddingZone, EnergyChartApiResponse)>,
) -> Result<(), WorkerError> {
    while let Some((zone, data)) = receiver.recv().await {
        let cloned_client = client.clone();
        let parsed_data = parse_pricing_data(&zone.to_tz(), &data)?;
        store_pricing_data(cloned_client, &zone, &parsed_data).await?;
    }
    Ok(())
}

pub fn parse_pricing_data(
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
        .table_name("electricity_pricing")
        .item("country", AttributeValue::S(bzn.to_country_string()))
        .item("date", AttributeValue::S(get_storage_date().to_string()))
        .item(
            "pricing_data",
            AttributeValue::S(serde_json::to_string(pricing).unwrap()),
        )
        .send()
        .await
        .inspect(|_| {
            info!(
                "Pricing data successfully inserted to DynamoDB for zone: {}",
                bzn
            );
        })
        .map_err(|e| {
            error!(?e, "Inserting pricing data to DynamoDB failed");
            Box::new(e.into_service_error())
        })?;

    Ok(())
}
