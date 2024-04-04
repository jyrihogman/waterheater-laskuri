use std::sync::Arc;

use aws_lambda_events::event::cloudwatch_events::CloudWatchEvent;
use aws_sdk_dynamodb as dynamodb;
use dynamodb::{operation::put_item::PutItemError, types::AttributeValue};
use lambda_runtime::{run, service_fn, tracing, Error, LambdaEvent};

use chrono::{offset::LocalResult, DateTime, Days, TimeZone};
use chrono_tz::{Europe::Helsinki, Tz};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::form_urlencoded;

// Full example URL for the call
// https://api.energy-charts.info/price?bzn=FI&start=2024-06-01T00%3A00%2B01%3A00&end=2024-06-01T23%3A45%2B01%3A00
static BASE_URL: &str = "https://api.energy-charts.info/price?bzn=FI";

#[derive(Error, Debug)]
enum ApplicationError {
    #[error("Error from API: {0}")]
    Api(#[from] reqwest::Error),
    #[error("Error parsing data: {0}")]
    Parse(String),
    #[error("DynamoDB Put Item operation failed: {0}")]
    DatabaseError(#[from] Box<PutItemError>),
}

#[derive(Debug, Deserialize)]
struct EnergyChartApiResponse {
    pub unix_seconds: Arc<[u32]>,
    pub price: Arc<[f32]>,
}

#[derive(Debug, Serialize)]
struct HourlyPrice(DateTime<Tz>, f32);

async fn get_electricity_pricing() -> Result<EnergyChartApiResponse, reqwest::Error> {
    let start_date = Helsinki.from_utc_datetime(&chrono::Utc::now().naive_utc());
    println!("start_date: {}", start_date);
    let end_date = match start_date.checked_add_days(Days::new(1)) {
        Some(d) => d,
        None => start_date,
    };

    let urli: String = format!(
        "{}&start={}&end={}",
        BASE_URL,
        form_urlencoded::byte_serialize(start_date.to_rfc3339().as_bytes()).collect::<String>(),
        form_urlencoded::byte_serialize(end_date.to_rfc3339().as_bytes()).collect::<String>()
    );
    println!("{}", urli);

    reqwest::get(urli)
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
    pricing_data: &EnergyChartApiResponse,
) -> Result<Vec<HourlyPrice>, ApplicationError> {
    let mut kwh_pricing_data: Vec<HourlyPrice> = vec![];

    for (index, price) in pricing_data.price.iter().enumerate() {
        let unix_timestamp = pricing_data.unix_seconds[index] as i64;

        let date_time = match Helsinki.timestamp_opt(unix_timestamp, 0) {
            LocalResult::Single(d) => d,
            _ => {
                return Err(ApplicationError::Parse(
                    "Formatting unix timestamp to datetime failed".to_string(),
                ))
            }
        };

        kwh_pricing_data.push(HourlyPrice(date_time, price / 1000_f32))
    }

    Ok(kwh_pricing_data)
}

async fn store_pricing_data(pricing_data: &Vec<HourlyPrice>) -> Result<(), Box<PutItemError>> {
    let config = aws_config::load_from_env().await;
    let client = dynamodb::Client::new(&config);

    client
        .put_item()
        .table_name("electricity_pricing_info")
        .item("PricingId", AttributeValue::S("pricing".to_string()))
        .item(
            "Data",
            AttributeValue::S(serde_json::to_string(&pricing_data).unwrap()),
        )
        .send()
        .await
        .inspect(|_| {
            println!("Pricing successfully inserted to DynamoDB");
        })
        .map_err(|e| Box::new(e.into_service_error()))?;

    Ok(())
}

async fn function_handler(_event: LambdaEvent<CloudWatchEvent>) -> Result<(), ApplicationError> {
    let pricing_data = get_electricity_pricing().await?;
    let parsed_pricing_data = parse_pricing_data(&pricing_data)?;

    store_pricing_data(&parsed_pricing_data).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    run(service_fn(function_handler)).await
}
