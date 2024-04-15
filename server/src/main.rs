use std::{net::SocketAddr, time::Duration};

use axum::Router;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::v2::handler as cheapest_period;
use crate::{v1::router::v1_routes, v2::router::v2_routes};

mod common;
mod http;
mod tests;
mod v1;
mod v2;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    #[derive(OpenApi)]
    #[openapi(
        paths(cheapest_period::handle_enable_water_heater),
        components(
            schemas(wh_core::types::BiddingZone)
        ),
        tags(
            (name = "waterheater-calc", description = "Easy-to-use API designed to be used with ready-made Shelly scripts for controlling 
                for example a waterheater to be turned on at certain hours of the day.")
        )
    )]
    struct ApiDoc;

    // Allow bursts of 10 requests per IP
    // Swagger UI needs 5 requests to complete in itself
    // Replenish one every two seconds
    // Config needs to be Boxed as Axum reqires all layers to implement clone
    let governor_conf = Box::new(
        GovernorConfigBuilder::default()
            .per_second(2)
            .burst_size(10)
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
        .nest("/api/v1", v1_routes())
        .nest("/api/v2", v2_routes())
        .merge(
            SwaggerUi::new("/api/v2/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()),
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
