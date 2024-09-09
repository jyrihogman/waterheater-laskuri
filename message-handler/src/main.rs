use core::fmt;
use std::{env, error::Error as StdError};

use aws_lambda_events::sqs::SqsMessage;
use aws_sdk_scheduler as scheduler;
use chrono::{DateTime, Duration, Utc};
use lambda_runtime::{run, service_fn, tower::BoxError, tracing, Error, LambdaEvent};
use scheduler::types::{
    ActionAfterCompletion, FlexibleTimeWindow, FlexibleTimeWindowMode, ScheduleState, Target,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct MessageBody {
    retry_attempt: u16,
    retry_time: DateTime<Utc>,
}

#[derive(Debug)]
struct HandlingError(String);

impl fmt::Display for HandlingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl StdError for HandlingError {}

fn get_new_message(message_body: &MessageBody) -> MessageBody {
    MessageBody {
        retry_attempt: message_body.retry_attempt + 1,
        retry_time: message_body.retry_time + Duration::minutes(15),
    }
}

async fn handle_message_scheduling(event: LambdaEvent<SqsMessage>) -> Result<(), BoxError> {
    let config = aws_config::load_from_env().await;
    let client = scheduler::Client::new(&config);

    let default_message = MessageBody {
        retry_attempt: 1,
        retry_time: Utc::now(),
    };

    let message_string = event.payload.body.unwrap_or_else(|| {
        println!("body not available");
        serde_json::to_string(&default_message).unwrap()
    });

    let message_body = serde_json::from_str::<MessageBody>(&message_string)?;

    if message_body.retry_attempt > 5 {
        eprintln!("MaxRetryAttemptsExceeded");
        return Err(Box::new(HandlingError(
            "MaxRetryAttemptsExceeded".to_string(),
        )));
    }

    let new_message = get_new_message(&message_body);
    let date_time_string = new_message.retry_time.to_string();
    let schedule = format!("at({date_time_string})");

    let target = Target::builder()
        .set_arn(Option::Some(env::var("queueArn")?))
        .set_role_arn(Option::Some(env::var("roleArn")?))
        .set_input(Option::Some(serde_json::to_string(&new_message)?))
        .build()?;

    client
        .create_schedule()
        .set_name(Option::Some("GetElectricityPricingSchedule".to_string()))
        .set_schedule_expression(Option::Some(schedule.to_string()))
        .set_flexible_time_window(Option::Some(
            FlexibleTimeWindow::builder()
                .mode(FlexibleTimeWindowMode::Off)
                .build()?,
        ))
        .set_state(Option::Some(ScheduleState::Enabled))
        .set_action_after_completion(Option::Some(ActionAfterCompletion::Delete))
        .set_target(Option::Some(target));

    println!("Schedule created for {date_time_string}");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    run(service_fn(handle_message_scheduling)).await
}
