#![cfg(test)]

use chrono::{FixedOffset, TimeZone, Timelike};
use std::sync::Arc;

use crate::{
    db::Pricing,
    service::{calculate_cheapest_start_time, get_filtered_pricing},
};

fn create_static_pricing_with_hour(hour: u32) -> Pricing {
    let offset = FixedOffset::east_opt(0).unwrap();
    let date_time = offset.with_ymd_and_hms(2024, 4, 8, hour, 0, 0).unwrap();

    Pricing {
        date_time,
        price_no_tax: 0.5,
    }
}

fn create_pricing_with_hour(hour: u32, price: f32) -> Pricing {
    let offset = FixedOffset::east_opt(0).unwrap();
    let date_time = offset.with_ymd_and_hms(2024, 4, 8, hour, 0, 0).unwrap();
    Pricing {
        date_time,
        price_no_tax: price,
    }
}

#[test]
fn test_get_filtered_pricing_normal_period() {
    let pricing_data = vec![
        create_static_pricing_with_hour(10),
        create_static_pricing_with_hour(15),
        create_static_pricing_with_hour(20),
    ];
    let pricing = Arc::from(pricing_data.into_boxed_slice());
    let starting_hour = 12;
    let ending_hour = 18;

    let filtered_pricing = get_filtered_pricing(&pricing, &starting_hour, &ending_hour);
    assert_eq!(filtered_pricing.len(), 1);
    assert_eq!(filtered_pricing[0].date_time.hour(), 15);
}

#[test]
fn test_get_filtered_pricing_cross_midnight() {
    let pricing_data = vec![
        create_static_pricing_with_hour(22),
        create_static_pricing_with_hour(23),
        create_static_pricing_with_hour(0),
        create_static_pricing_with_hour(1),
        create_static_pricing_with_hour(2),
        create_static_pricing_with_hour(3),
        create_static_pricing_with_hour(4),
        create_static_pricing_with_hour(5),
    ];
    let pricing = Arc::from(pricing_data.into_boxed_slice());
    let starting_hour = 22;
    let ending_hour = 5;

    let filtered_pricing = get_filtered_pricing(&pricing, &starting_hour, &ending_hour);
    assert_eq!(filtered_pricing.len(), 7);
    assert_eq!(filtered_pricing[0].date_time.hour(), 22);
    assert!(filtered_pricing.iter().any(|&p| p.date_time.hour() == 2));
}

#[test]
fn test_calculate_cheapest_start_time() {
    let pricing_data = vec![
        create_pricing_with_hour(22, 0.1),
        create_pricing_with_hour(23, 0.2),
        create_pricing_with_hour(0, 0.3),
        create_pricing_with_hour(1, 0.5),
        create_pricing_with_hour(2, 0.1),
        create_pricing_with_hour(3, 0.1),
        create_pricing_with_hour(4, 0.1),
        create_pricing_with_hour(5, 0.1),
    ];

    let pricing_refs: Vec<&Pricing> = pricing_data.iter().collect();

    let start_time = calculate_cheapest_start_time(pricing_refs, 6);

    assert_eq!(
        start_time,
        Some(
            FixedOffset::east_opt(0)
                .unwrap()
                .with_ymd_and_hms(2024, 4, 8, 0, 0, 0)
                .unwrap()
        )
    )
}

#[test]
fn test_calculate_cheapest_start_time_before_midnight() {
    let pricing_data = vec![
        create_pricing_with_hour(22, 0.01),
        create_pricing_with_hour(23, 0.02),
        create_pricing_with_hour(0, 0.004),
        create_pricing_with_hour(1, 0.1),
        create_pricing_with_hour(2, 0.2),
        create_pricing_with_hour(3, 0.6),
        create_pricing_with_hour(4, 0.7),
        create_pricing_with_hour(5, 0.8),
        create_pricing_with_hour(6, 0.9),
        create_pricing_with_hour(7, 0.6),
    ];
    let pricing_refs: Vec<&Pricing> = pricing_data.iter().collect();

    let start_time = calculate_cheapest_start_time(pricing_refs, 6);

    assert_eq!(
        start_time,
        Some(
            FixedOffset::east_opt(0)
                .unwrap()
                .with_ymd_and_hms(2024, 4, 8, 22, 0, 0)
                .unwrap()
        )
    );
}
