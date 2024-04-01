use chrono::{DateTime, Duration, FixedOffset, Timelike};
use serde::Deserialize;
use std::sync::Arc;

pub trait TimeProvider {
    fn now(&self) -> chrono::DateTime<chrono::Local>;
}

pub struct RealTimeProvider;

impl TimeProvider for RealTimeProvider {
    fn now(&self) -> chrono::DateTime<chrono::Local> {
        chrono::Local::now()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct Pricing {
    pub date_time: String,
    pub price_no_tax: f32,
}

fn is_within_period(
    current: &DateTime<FixedOffset>,
    start: &DateTime<FixedOffset>,
    end: &DateTime<FixedOffset>,
) -> bool {
    if start <= end {
        // Normal period, not crossing midnight
        current >= start && current <= end
    } else {
        // Period crosses midnight, adjust logic
        current >= start || current <= end
    }
}

fn get_filtered_pricing<'a>(
    pricing: &'a Arc<[Pricing]>,
    starting_hour: &'a u32,
    ending_hour: &'a u32,
) -> Vec<(DateTime<FixedOffset>, &'a Pricing)> {
    pricing
        .iter()
        .filter_map(|p| {
            DateTime::parse_from_rfc3339(&p.date_time)
                .ok()
                .filter(|dt| {
                    let hour = &dt.hour();
                    if starting_hour < ending_hour {
                        // Normal period, not crossing midnight
                        hour >= starting_hour && hour < ending_hour
                    } else {
                        // Period crosses midnight
                        hour >= starting_hour || hour < ending_hour
                    }
                })
                .map(|dt| (dt, p))
        })
        .collect()
}

fn calculate_cheapest_start_time(
    pricing: &[(DateTime<FixedOffset>, &Pricing)],
    hours: u32,
) -> Option<DateTime<FixedOffset>> {
    let mut cheapest_sequence_start: Option<DateTime<FixedOffset>> = None;
    let mut min_cost = f32::MAX;

    for window in pricing.windows(hours as usize) {
        let total_cost: f32 = window.iter().map(|(_, p)| p.price_no_tax).sum();
        if total_cost < min_cost {
            min_cost = total_cost;
            cheapest_sequence_start = Some(window.first().unwrap().0); // Correct access here
        }
    }

    cheapest_sequence_start
}

pub fn should_enable<T: TimeProvider>(
    time_provider: T,
    pricing: Arc<[Pricing]>,
    hours: u32,
    starting_hour: u32,
    ending_hour: u32,
) -> bool {
    let current_time = time_provider.now();
    let current_hour = current_time.hour();

    // Adjust the logic for checking if current time is within the operation hours
    let is_within_operating_hours = if starting_hour < ending_hour {
        current_hour >= starting_hour && current_hour < ending_hour
    } else {
        // Handling the cross-midnight scenario
        current_hour >= starting_hour || current_hour < ending_hour
    };

    if !is_within_operating_hours {
        return false;
    }
    let filtered_pricing = get_filtered_pricing(&pricing, &starting_hour, &ending_hour);

    println!("{:?}", filtered_pricing);

    if filtered_pricing.is_empty() || filtered_pricing.len() < hours as usize {
        return false;
    }

    let cheapest_sequence_start = calculate_cheapest_start_time(&filtered_pricing, hours);

    if let Some(start) = cheapest_sequence_start {
        let end = start + Duration::hours(i64::from(hours));
        return current_time >= start && current_time <= end;
    }

    false
}
