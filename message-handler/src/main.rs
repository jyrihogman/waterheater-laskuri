use core::fmt;
use rand::Rng;
use std::{env, error::Error as StdError};

use aws_lambda_events::sqs::{SqsEvent, SqsMessage};
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
    #[serde(default = "default_retry_time")]
    retry_time: DateTime<Utc>,
}

fn default_retry_time() -> DateTime<Utc> {
    Utc::now()
}

#[derive(Debug)]
struct HandlingError(String);

impl fmt::Display for HandlingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl StdError for HandlingError {}

fn calculate_retry_delay(retry_attempt: u16) -> Duration {
    let base_delay = Duration::minutes(5);
    let max_delay = Duration::hours(1);

    let exponent = retry_attempt - 1;
    let mut delay = base_delay * 2_i32.pow(exponent.into());

    if delay > max_delay {
        delay = max_delay;
    }

    let mut rng = rand::thread_rng();
    let jitter_ms = rng.gen_range(0..delay.num_milliseconds());

    Duration::milliseconds(jitter_ms)
}

fn get_new_message(message: Option<&SqsMessage>) -> MessageBody {
    match message
        .and_then(|msg| msg.body.as_ref())
        .and_then(|b| serde_json::from_str::<MessageBody>(b).ok())
    {
        Some(new_message) => MessageBody {
            retry_attempt: new_message.retry_attempt + 1,
            retry_time: Utc::now() + calculate_retry_delay(new_message.retry_attempt),
        },
        None => MessageBody {
            retry_attempt: 1,
            retry_time: Utc::now() + Duration::minutes(5),
        },
    }
}

async fn handle_message_scheduling(event: LambdaEvent<SqsEvent>) -> Result<(), BoxError> {
    let config = aws_config::load_from_env().await;
    let client = scheduler::Client::new(&config);

    println!("{:?}", event);

    let message_body = get_new_message(event.payload.records.first());

    if message_body.retry_attempt > 5 {
        eprintln!("MaxRetryAttemptsExceeded");
        return Err(Box::new(HandlingError(
            "MaxRetryAttemptsExceeded".to_string(),
        )));
    }

    let date_time_string = message_body
        .retry_time
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    let schedule_format_string = date_time_string.trim_end_matches('Z');
    let schedule = format!("at({schedule_format_string})");

    let target = Target::builder()
        .arn(env::var("queueArn")?)
        .role_arn(env::var("roleArn")?)
        .input(serde_json::to_string(&message_body)?)
        .build()?;

    let formatted_time = message_body.retry_time.timestamp();

    let response = client
        .create_schedule()
        .name(format!("GetPricingSchedule-{formatted_time}"))
        .schedule_expression(&schedule)
        .flexible_time_window(
            FlexibleTimeWindow::builder()
                .mode(FlexibleTimeWindowMode::Off)
                .build()?,
        )
        .state(ScheduleState::Enabled)
        .action_after_completion(ActionAfterCompletion::Delete)
        .target(target)
        .send()
        .await?;

    println!("Schedule ({schedule}) created, response: {:?}", response);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    run(service_fn(handle_message_scheduling)).await
}
