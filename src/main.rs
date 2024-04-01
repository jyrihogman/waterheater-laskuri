use std::sync::Arc;

use app::{Pricing, TimeProvider};
use axum::{extract::Path, http::StatusCode, response::IntoResponse, routing::get, Router};

use crate::app::{should_enable, RealTimeProvider};

mod app;

static PRICING_URL: &str = "https://api.spot-hinta.fi/TodayAndDayForward";

async fn get_water_heater_hours() -> Result<Arc<[Pricing]>, reqwest::Error> {
    reqwest::get(PRICING_URL)
        .await?
        .json::<Arc<[Pricing]>>()
        .await
}

async fn handle_enable_water_heater(
    Path((hours, starting_hour, ending_hour)): Path<(u32, u32, u32)>,
) -> impl IntoResponse {
    println!("Handling request for {} hours", hours);
    let pricing = match get_water_heater_hours().await {
        Ok(p) => p,
        Err(e) => {
            println!("Error: {}", e);
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };

    let is_enabled = should_enable(RealTimeProvider, pricing, hours, starting_hour, ending_hour);

    if is_enabled {
        return StatusCode::OK;
    }

    StatusCode::BAD_REQUEST
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        // Health check on root path
        .route("/", get(StatusCode::OK))
        .route(
            "/waterheater/hours/:hours/starting/:starting_hour/ending/:ending_hour",
            get(handle_enable_water_heater),
        );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8001").await.unwrap();
    println!("Server Listening");

    axum::serve(listener, app).await.unwrap();
}

struct MockTimeProvider {
    mock_now: chrono::DateTime<chrono::Local>,
}

impl TimeProvider for MockTimeProvider {
    fn now(&self) -> chrono::DateTime<chrono::Local> {
        self.mock_now
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::sync::Arc;

    // Helper function to create mock pricing data
    fn mock_pricing_data() -> Arc<[Pricing]> {
        vec![
            Pricing {
                date_time: "2024-03-30T22:00:00+02:00".to_string(),
                price_no_tax: 1.0,
            },
            Pricing {
                date_time: "2024-03-30T23:00:00+02:00".to_string(),
                price_no_tax: 1.0,
            },
            Pricing {
                date_time: "2024-03-31T00:00:00+02:00".to_string(),
                price_no_tax: 1.0,
            },
            Pricing {
                date_time: "2024-03-31T01:00:00+02:00".to_string(),
                price_no_tax: 2.0,
            },
            Pricing {
                date_time: "2024-03-31T02:00:00+02:00".to_string(),
                price_no_tax: 3.0,
            },
            Pricing {
                date_time: "2024-03-31T03:00:00+02:00".to_string(),
                price_no_tax: 3.0,
            },
            Pricing {
                date_time: "2024-03-31T04:00:00+02:00".to_string(),
                price_no_tax: 3.0,
            },
            Pricing {
                date_time: "2024-03-31T05:00:00+02:00".to_string(),
                price_no_tax: 3.0,
            },
            Pricing {
                date_time: "2024-03-31T06:00:00+02:00".to_string(),
                price_no_tax: 3.0,
            },
            Pricing {
                date_time: "2024-03-31T02:00:00+02:00".to_string(),
                price_no_tax: 3.0,
            },
        ]
        .into()
    }

    #[tokio::test]
    async fn test_should_enable_heating_within_cheapest_sequence() {
        let pricing = mock_pricing_data();
        let hours = 2;
        let starting_hour = 0;
        let ending_hour = 3;

        let mock_time_provider = MockTimeProvider {
            mock_now: chrono::Local.ymd(2024, 3, 31).and_hms(0, 30, 0),
        };

        let enable = should_enable(
            mock_time_provider,
            pricing,
            hours,
            starting_hour,
            ending_hour,
        );
        assert!(
            enable,
            "Heating should be enabled within the cheapest sequence"
        );
    }

    #[tokio::test]
    async fn test_should_enable_heating_within_cheapest_sequence_before_midnight() {
        let pricing = mock_pricing_data();
        let hours = 2;
        let starting_hour = 22;
        let ending_hour = 6;

        let mock_time_provider = MockTimeProvider {
            mock_now: chrono::Local.ymd(2024, 3, 30).and_hms(22, 30, 0),
        };

        let enable = should_enable(
            mock_time_provider,
            pricing,
            hours,
            starting_hour,
            ending_hour,
        );
        assert!(
            enable,
            "Heating should be enabled within the cheapest sequence"
        );
    }

    #[tokio::test]
    async fn test_should_not_enable_heating_outside_cheapest_sequence() {
        let pricing = mock_pricing_data();
        let hours = 2;
        let starting_hour = 0;
        let ending_hour = 3;

        let mock_time_provider = MockTimeProvider {
            mock_now: chrono::Local.ymd(2024, 3, 31).and_hms(6, 0, 0),
        };

        let enable = should_enable(
            mock_time_provider,
            pricing,
            hours,
            starting_hour,
            ending_hour,
        );
        assert!(
            !enable,
            "Heating should not be enabled outside the cheapest sequence"
        );
    }
}
