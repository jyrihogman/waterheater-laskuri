use aws_sdk_dynamodb::error::BoxError;
use chrono::{offset::LocalResult, DateTime, TimeZone};
use chrono_tz::Tz;

use wh_core::types::BiddingZone;

use crate::time_provider::TimeProvider;
use crate::types::{EnergyChartApiResponse, WorkerError};

pub fn unix_timestamp_to_datetime(tz: &Tz, unix_timestamp: &u32) -> Result<DateTime<Tz>, BoxError> {
    match tz.timestamp_opt(unix_timestamp.to_owned() as i64, 0) {
        LocalResult::Single(d) => Ok(d),
        _ => Err(Box::new(WorkerError::Parse(
            "Formatting unix timestamp to datetime failed".to_string(),
        ))),
    }
}

pub fn has_new_results<T: TimeProvider>(
    data: EnergyChartApiResponse,
    time_provider: &T,
) -> Result<bool, BoxError> {
    let newest_result = match data.unix_seconds.last() {
        Some(&timestamp) => unix_timestamp_to_datetime(&BiddingZone::FI.to_tz(), &timestamp)?,
        None => return Err("Could not convert unix timestampt to datetime".into()),
    };

    Ok(newest_result.date_naive()
        > time_provider
            .now()
            .with_timezone(&BiddingZone::FI.to_tz())
            .date_naive())
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use std::sync::Arc;

    pub struct MockTimeProvider {
        mock_time: DateTime<Utc>,
    }

    impl MockTimeProvider {
        pub fn new(mock_time: DateTime<Utc>) -> Self {
            MockTimeProvider { mock_time }
        }
    }

    impl TimeProvider for MockTimeProvider {
        fn now(&self) -> DateTime<Utc> {
            self.mock_time
        }
    }

    use super::*;

    #[test]
    fn test_has_new_results_positive() {
        let fixed_time = Utc.ymd(2024, 9, 17).and_hms(12, 0, 0);
        let mock_provider = MockTimeProvider::new(fixed_time);

        // Call the function with the mock time provider
        let data = EnergyChartApiResponse {
            unix_seconds: Arc::new([1726689600]),
            price: Arc::new([38.43]),
        };

        let result = has_new_results(data, &mock_provider).unwrap();

        assert!(result, "Expected new results to be available");
    }

    #[test]
    fn test_has_new_results_negative() {
        let fixed_time = Utc.ymd(2024, 9, 18).and_hms(12, 0, 0);
        let mock_provider = MockTimeProvider::new(fixed_time);

        // Call the function with the mock time provider
        let data = EnergyChartApiResponse {
            unix_seconds: Arc::new([1726689600]),
            price: Arc::new([38.43]),
        };

        let result = has_new_results(data, &mock_provider).unwrap();

        assert!(!result, "Expected new results not to be available");
    }
}
