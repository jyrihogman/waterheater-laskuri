use std::sync::Arc;

use aws_sdk_dynamodb::operation::put_item::PutItemError;
use chrono::DateTime;
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::task::JoinError;

#[derive(Error, Debug)]
pub enum WorkerError {
    #[error("Error from API: {0}")]
    Api(#[from] reqwest::Error),
    #[error("New pricing data not available")]
    Data(String),
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
pub struct HourlyPrice(pub DateTime<Tz>, pub f32);
