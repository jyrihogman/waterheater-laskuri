use std::{net::SocketAddr, time::Duration};

use axum::{
    extract::{ConnectInfo, Path},
    http::StatusCode,
    response::IntoResponse,
};

use crate::v1::service::is_water_heater_enabled_for_current_hour;

pub async fn handle_enable_water_heater(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path((hours, starting_hour, ending_hour)): Path<(u32, u32, u32)>,
) -> impl IntoResponse {
    let is_enabled =
        is_water_heater_enabled_for_current_hour(hours, starting_hour, ending_hour).await;

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
