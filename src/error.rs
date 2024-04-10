use aws_sdk_dynamodb::operation::get_item::GetItemError;
use aws_sdk_ssm::{error::SdkError, operation::get_parameters_by_path::GetParametersByPathError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApplicationError {
    #[error("Error from API: {0}")]
    Api(#[from] reqwest::Error),
    #[error("Error parsing data: {0}")]
    Service(String),
    #[error("DynamoDB Put Item operation failed: {0}")]
    Database(#[from] Box<GetItemError>),
    #[error("SSM Get Parameters operation failure: {0}")]
    Ssm(#[from] SdkError<GetParametersByPathError>),
}
