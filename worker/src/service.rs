use chrono::DateTime;
use chrono::{offset::LocalResult, Duration, TimeZone, Utc};
use chrono_tz::Europe::Helsinki;
use chrono_tz::Tz;
use lambda_runtime::tower::BoxError;

use crate::types::WorkerError;

pub fn unix_timestampt_to_datetime(
    tz: &Tz,
    unix_timestamp: &u32,
) -> Result<DateTime<Tz>, BoxError> {
    match tz.timestamp_opt(unix_timestamp.to_owned() as i64, 0) {
        LocalResult::Single(d) => Ok(d),
        _ => Err(Box::new(WorkerError::Parse(
            "Formatting unix timestamp to datetime failed".to_string(),
        ))),
    }
}

pub fn has_new_results(date_time: DateTime<Tz>) -> bool {
    let utc_now = Utc::now();

    let helsinki_time = utc_now.with_timezone(&Helsinki) + Duration::days(1);

    date_time.date_naive() < helsinki_time.date_naive()
}
