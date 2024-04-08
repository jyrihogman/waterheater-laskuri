use axum::{extract::Path, http::StatusCode, response::IntoResponse, routing::get, Router};

use crate::service::is_water_heater_enabled_for_current_hour;

mod db;
mod error;
mod service;

mod tests;

async fn handle_enable_water_heater(
    Path((hours, starting_hour, ending_hour)): Path<(u32, u32, u32)>,
) -> impl IntoResponse {
    if is_water_heater_enabled_for_current_hour(hours, starting_hour, ending_hour).await {
        return StatusCode::OK;
    }

    StatusCode::BAD_REQUEST
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        // Health check on root path
        .route("/", get(StatusCode::OK))
        .route(
            "/waterheater/hours/:hours/starting/:starting_hour/ending/:ending_hour",
            get(handle_enable_water_heater),
        );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8001").await.unwrap();
    println!("Server Listening");

    axum::serve(listener, app).await.unwrap();
}
