use std::{str::FromStr, sync::Arc};

use aws_sdk_dynamodb::types::AttributeValue;
use chrono::{DateTime, FixedOffset};
use serde::Deserialize;
use serde_dynamo::aws_sdk_dynamodb_1::from_item;

use crate::error::ApplicationError;

#[derive(Debug, Deserialize)]
pub struct Pricing {
    pub date_time: DateTime<FixedOffset>,
    pub price_no_tax: f32,
}

#[derive(Debug, Deserialize)]
struct DynamoData(String, f32);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct DynamoItem {
    data: String,
}

pub async fn get_electricity_pricing() -> Result<Arc<[Pricing]>, Box<dyn std::error::Error>> {
    let config = aws_config::load_from_env().await;
    let client = aws_sdk_dynamodb::Client::new(&config);

    let get_item_output = client
        .get_item()
        .table_name("electricity_pricing_info")
        .key("PricingId", AttributeValue::S("pricing".to_string()))
        .send()
        .await?;

    let deserialized_row = from_item::<DynamoItem>(
        get_item_output
            .item
            .ok_or(ApplicationError::Service("Item not found".to_string()))?,
    )
    .map_err(|e| ApplicationError::Service(e.to_string()))?;

    let items = serde_json::from_str::<Vec<DynamoData>>(&deserialized_row.data).unwrap();

    Ok(items
        .iter()
        .map(|p| Pricing {
            date_time: DateTime::from_str(&p.0).unwrap(),
            price_no_tax: p.1,
        })
        .collect())
}
