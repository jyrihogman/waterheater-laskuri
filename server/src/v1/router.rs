use axum::routing::get;
use axum::Router;

use crate::http::not_found;

use super::handler::handle_enable_water_heater;

pub fn v1_routes() -> Router {
    Router::new()
        .route(
            "/waterheater/hours/:hours/starting/:starting_hour/ending/:ending_hour",
            get(handle_enable_water_heater),
        )
        .fallback(not_found)
}
