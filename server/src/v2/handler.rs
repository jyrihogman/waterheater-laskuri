use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Path, Query},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;

use wh_core::types::BiddingZone;

use super::service::is_water_heater_enabled_for_current_hour;

#[derive(Deserialize)]
pub struct QueryParams {
    hours: u32,
    start: u32,
    end: u32,
}

/// API returns only 200 and 400 for compatibility purposes
/// Shelly devices are used in Finland to control waterheaters etc,
/// and the basic scripts control the devices according to HTTP status codes.
/// In the future I will most likely rewrite some of the scripts to allow
/// better & more versatile endpoint design.
#[utoipa::path(
    get,
    path = "/api/v2/waterheater/country/{country_code}/cheapest-period",
    responses(
        (status = 200, description = "Current hour is withing the cheapest period of electricity price"),
        (status = 400, description = "Current hour is not in the cheapest period of electricity price"),
    ),
    params(
        ("country_code" = BiddingZone, Path, description = "Country code"),
        ("hours" = u32, Query, description = "Number of hours in the period"),
        ("start" = u32, Query, description = "First hour of the period in 24h format"),
        ("end" = u32, Query, description = "The hour when the period ends in 24h format")
    ),
)]
pub async fn handle_enable_water_heater(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(country_code): Path<BiddingZone>,
    Query(params): Query<QueryParams>,
) -> impl IntoResponse {
    let is_enabled = is_water_heater_enabled_for_current_hour(
        country_code,
        params.hours,
        params.start,
        params.end,
    )
    .await;

    if is_enabled {
        println!(
            "Waterheater enabled at {} ({} hours starting at {})",
            addr.ip(),
            params.hours,
            params.start
        );
        return StatusCode::OK;
    }

    StatusCode::BAD_REQUEST
}
