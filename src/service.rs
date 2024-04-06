use chrono::TimeZone;
use chrono::{DateTime, Duration, FixedOffset, Timelike};
use chrono_tz::Europe::Helsinki;
use chrono_tz::Tz;

use std::sync::Arc;

use crate::db::{get_electricity_pricing, Pricing};

fn get_filtered_pricing<'a>(
    pricing: &'a Arc<[Pricing]>,
    starting_hour: &'a u32,
    ending_hour: &'a u32,
) -> Vec<&'a Pricing> {
    pricing
        .iter()
        .filter(|p| {
            let hour = &p.date_time.hour();
            if starting_hour < ending_hour {
                // Normal period, not crossing midnight
                hour >= starting_hour && hour < ending_hour
            } else {
                // Period crosses midnight
                hour >= starting_hour || hour < ending_hour
            }
        })
        .collect()
}

fn calculate_cheapest_start_time(
    pricing: Vec<&Pricing>,
    hours: u32,
) -> Option<DateTime<FixedOffset>> {
    let mut cheapest_sequence_start: Option<DateTime<FixedOffset>> = None;
    let mut min_cost = f32::MAX;

    for window in pricing.windows(hours as usize) {
        let total_cost: f32 = window.iter().map(|p| p.price_no_tax).sum();
        if total_cost < min_cost {
            min_cost = total_cost;
            cheapest_sequence_start = Some(window.first().unwrap().date_time);
        }
    }

    cheapest_sequence_start
}

fn is_within_operating_hours(
    starting_hour: u32,
    ending_hour: u32,
    current_time: DateTime<Tz>,
) -> bool {
    let current_hour = current_time.hour();

    if starting_hour < ending_hour {
        return current_hour >= starting_hour && current_hour < ending_hour;
    }

    current_hour >= starting_hour || current_hour < ending_hour
}

pub async fn is_water_heater_enabled_for_current_hour(
    hours: u32,
    starting_hour: u32,
    ending_hour: u32,
) -> bool {
    let pricing = match get_electricity_pricing().await {
        Ok(p) => p,
        Err(e) => {
            println!("Error: {}", e);
            return false;
        }
    };

    let current_time = Helsinki.from_utc_datetime(&chrono::Utc::now().naive_utc());

    if !is_within_operating_hours(starting_hour, ending_hour, current_time) {
        return false;
    }

    let filtered_pricing = get_filtered_pricing(&pricing, &starting_hour, &ending_hour);

    if filtered_pricing.is_empty() || filtered_pricing.len() < hours as usize {
        return false;
    }

    let cheapest_sequence_start = calculate_cheapest_start_time(filtered_pricing, hours);

    if let Some(start) = cheapest_sequence_start {
        let end = start + Duration::hours(i64::from(hours));
        return current_time >= start && current_time <= end;
    }

    false
}
