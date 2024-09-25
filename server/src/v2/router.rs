use axum::routing::get;
use axum::Router;

use crate::http::not_found;
use crate::AppState;

use super::handler::handle_enable_water_heater;

pub fn v2_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/waterheater/country/:country_code/cheapest-period",
            get(handle_enable_water_heater),
        )
        .fallback(not_found)
}
