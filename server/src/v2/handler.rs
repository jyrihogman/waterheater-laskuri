use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Path, Query},
    http::StatusCode,
    response::IntoResponse,
};

use crate::common::types::BiddingZone;

use super::service::is_water_heater_enabled_for_current_hour;

pub async fn handle_enable_water_heater(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(country_code): Path<BiddingZone>,
    Query((hours, starting_hour, ending_hour)): Query<(u32, u32, u32)>,
) -> impl IntoResponse {
    let is_enabled =
        is_water_heater_enabled_for_current_hour(country_code, hours, starting_hour, ending_hour)
            .await;

    if is_enabled {
        println!(
            "Waterheater enabled at {} ({} hours starting at {})",
            addr.ip(),
            hours,
            starting_hour
        );
        return StatusCode::OK;
    }

    StatusCode::BAD_REQUEST
}
