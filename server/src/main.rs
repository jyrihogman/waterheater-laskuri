use std::{net::SocketAddr, time::Duration};

use axum::{
    extract::{ConnectInfo, Path},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};
use service::is_water_heater_enabled_for_current_hour;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

mod db;
mod error;
mod service;
mod tests;

async fn handle_enable_water_heater(
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

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Allow bursts of five requests per IP
    // Replenish one every two seconds
    // Config needs to be Boxed as Axum reqires all layers to implement clone
    let governor_conf = Box::new(
        GovernorConfigBuilder::default()
            .per_second(2)
            .burst_size(5)
            .finish()
            .unwrap(),
    );

    let governor_limiter = governor_conf.limiter().clone();
    let interval = Duration::from_secs(60);

    std::thread::spawn(move || loop {
        std::thread::sleep(interval);
        governor_limiter.retain_recent();
    });

    let app = Router::new()
        .route("/", get(StatusCode::OK))
        .route(
            "/waterheater/hours/:hours/starting/:starting_hour/ending/:ending_hour",
            get(handle_enable_water_heater),
        )
        .layer(GovernorLayer {
            config: Box::leak(governor_conf),
        });

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8001").await.unwrap();
    println!("Server Listening");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
