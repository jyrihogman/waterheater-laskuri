use std::{net::SocketAddr, time::Duration};

use axum::Router;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

use crate::v1::router::v1_routes;

mod db;
mod error;
mod http;
mod tests;
mod v1;

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

    let app = Router::new().nest("/v1", v1_routes()).layer(GovernorLayer {
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
