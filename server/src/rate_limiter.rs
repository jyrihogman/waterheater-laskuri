use aws_sdk_dynamodb::{
    operation::update_item::UpdateItemError, types::AttributeValue, Client as DynamoDbClient,
    Error as DynamoError,
};
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use chrono::{Duration, Utc};
use tracing::error;

use crate::AppState;

pub async fn rate_limit(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let client_ip = extract_client_ip(&request).unwrap_or("unknown");

    let is_limited = match is_rate_limited(&state.dynamo_client, client_ip).await {
        Ok(limited) => limited,
        Err(e) => {
            error!("Rate limiter error: {}", e);
            false // Decide whether to fail open or closed
        }
    };

    if is_limited {
        error!("Rate limit exceeded for client IP: {}", client_ip);
        let mut response = Response::new("Too Many Requests".into());
        *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
        Ok(response)
    } else {
        Ok(next.run(request).await)
    }
}

async fn is_rate_limited(
    dynamodb_client: &DynamoDbClient,
    client_id: &str,
) -> Result<bool, DynamoError> {
    // Rate limiting constants
    let max_requests = 20;
    let window_seconds = 60;

    let now = Utc::now();
    let window_start = now.timestamp() - (now.timestamp() % window_seconds);

    // Set the expiration time for the item
    let expires_at = now + Duration::seconds(window_seconds * 2);

    let update_expression = "\
        SET \
            #request_count = if_not_exists(#request_count, :start) + :inc, \
            #window_start = :window_start, \
            #expires_at = :expires_at";

    let condition_expression =
        "attribute_not_exists(#request_count) OR #request_count <= :max_requests";

    let result = dynamodb_client
        .update_item()
        .table_name("waterheater_calc_rate_limits")
        .key("client_id", AttributeValue::S(client_id.to_string()))
        .update_expression(update_expression)
        .condition_expression(condition_expression)
        .expression_attribute_names("#request_count", "request_count")
        .expression_attribute_names("#window_start", "window_start")
        .expression_attribute_names("#expires_at", "expires_at")
        .expression_attribute_values(":inc", AttributeValue::N(1.to_string()))
        .expression_attribute_values(":start", AttributeValue::N(0.to_string()))
        .expression_attribute_values(":window_start", AttributeValue::N(window_start.to_string()))
        .expression_attribute_values(":max_requests", AttributeValue::N(max_requests.to_string()))
        .expression_attribute_values(
            ":expires_at",
            AttributeValue::N(expires_at.timestamp().to_string()),
        )
        .send()
        .await;

    match result {
        Ok(_) => Ok(false), // Not rate limited
        Err(e) => match e.into_service_error() {
            UpdateItemError::ConditionalCheckFailedException(e) => {
                error!("ConditionalCheckFailedException: {:?}", e);
                Ok(true)
            }
            err => {
                error!("Unhandled service error: {:?}", err);
                Ok(false)
            }
        },
    }
}

fn extract_client_ip<B>(request: &Request<B>) -> Option<&str> {
    request
        .headers()
        .get("X-Forwarded-For")
        .and_then(|value| value.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim())
}
