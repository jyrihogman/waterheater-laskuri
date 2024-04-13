use axum::routing::get;
use axum::Router;

use crate::http::not_found;

use super::handler::handle_enable_water_heater;

pub fn v2_routes() -> Router {
    Router::new()
        .route(
            "/waterheater/country/:country_code/hours/:hours/start/:starting_hour/end/:ending_hour",
            get(handle_enable_water_heater),
        )
        .fallback(not_found)
}
