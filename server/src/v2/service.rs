use chrono::TimeZone;
use chrono::{DateTime, Duration, FixedOffset, Timelike};
use chrono_tz::Tz;
use tracing::{error, info};

use std::sync::Arc;

use wh_core::time_provider::{self, SystemTimeProvider, TimeProvider};
use wh_core::types::BiddingZone;

use crate::common::db::{get_electricity_pricing_with_region, Pricing};

pub fn get_filtered_pricing<'a, T: TimeProvider>(
    time_provider: &T,
    country_code: &BiddingZone,
    pricing: &'a Arc<[Pricing]>,
    starting_hour: u32,
    ending_hour: u32,
) -> Vec<&'a Pricing> {
    let current_day = time_provider
        .now()
        .with_timezone(&country_code.to_tz())
        .date_naive();

    let current_hour = time_provider
        .now()
        .with_timezone(&country_code.to_tz())
        .hour();

    // If starting hour is larger than ending hour, it means we are crossing the night
    // Which means to accurately filter the pricing info for the period, we need to
    // also return the pricing info from the starting hour on the previous day, when
    // clock has for example reached 02:00. Example is starting hour 22 ending hour 07
    // To correctly calculate the cheapest period we need the pricing information from 22:00
    // yesterday to 07 today.
    pricing
        .iter()
        .filter(|p| {
            let pricing_hour = p.date_time.hour();
            let pricing_date = p.date_time.date_naive();

            if starting_hour < ending_hour {
                // Normal period, not crossing midnight, the date on the pricing information has to
                // match current date (we want to filter out yesterdays pricing info as it's not
                // relevant for the period)
                pricing_hour >= starting_hour
                    && pricing_hour < ending_hour
                    && pricing_date == current_day
            } else {
                // Period crosses midnight, starting hour (e.g. 22) is LARGER than ending hour
                // (e.g. 07). We need to include pricing data for any hours between 22 and 00 if
                // the date matches to today
                // OR ((if the pricing data hour is earlier than the periods ending hour and it's
                // for the current day AND current hour is smaller than the starting hour ) OR (if
                // the pricing hour is smaller than the ending hour AND the pricing date is ahead one
                // day compared to current day)
                (pricing_hour >= starting_hour && pricing_date == current_day)
                    || ((pricing_hour < ending_hour
                        && pricing_date == current_day
                        && current_hour < starting_hour)
                        || (pricing_hour < ending_hour
                            && pricing_date == current_day + Duration::days(1)))
            }
        })
        .collect()
}

pub fn calculate_cheapest_start_time(
    pricing: Vec<&Pricing>,
    hours: u32,
) -> Option<DateTime<FixedOffset>> {
    let mut cheapest_sequence_start: Option<DateTime<FixedOffset>> = None;
    let mut min_cost = 50_f32;

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
    dynamo_client: aws_sdk_dynamodb::Client,
    country_code: BiddingZone,
    hours: u32,
    starting_hour: u32,
    ending_hour: u32,
) -> bool {
    let pricing = match get_electricity_pricing_with_region(&country_code, dynamo_client).await {
        Ok(p) => p,
        Err(e) => {
            error!("Error retrieving pricing from DynamoDB: {:?}", e);
            return false;
        }
    };

    let filtered_pricing = get_filtered_pricing::<SystemTimeProvider>(
        &time_provider::SystemTimeProvider,
        &country_code,
        &pricing,
        starting_hour,
        ending_hour,
    );

    if filtered_pricing.is_empty() || filtered_pricing.len() < hours as usize {
        return false;
    }

    let cheapest_sequence_start = calculate_cheapest_start_time(filtered_pricing, hours);

    info!(
        "Cheapest start time: {:?} for {} hours starting from {} and ending at {}",
        cheapest_sequence_start, hours, starting_hour, ending_hour
    );

    let current_time = country_code
        .to_tz()
        .from_utc_datetime(&chrono::Utc::now().naive_utc());

    if !is_within_operating_hours(starting_hour, ending_hour, current_time) {
        info!(
            starting_hour,
            ending_hour, "Current time is not within operation hours"
        );
        return false;
    }

    if let Some(start) = cheapest_sequence_start {
        let end = start + Duration::hours(i64::from(hours));
        return current_time >= start && current_time <= end;
    }

    false
}
