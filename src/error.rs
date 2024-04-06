use aws_sdk_dynamodb::operation::get_item::GetItemError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApplicationError {
    #[error("Error from API: {0}")]
    Api(#[from] reqwest::Error),
    #[error("Error parsing data: {0}")]
    Service(String),
    #[error("DynamoDB Put Item operation failed: {0}")]
    Database(#[from] Box<GetItemError>),
}
