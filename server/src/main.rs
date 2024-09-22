use std::env::set_var;

use axum::Router;
use lambda_http::{run, Error};

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::rate_limit::RateLimit;
use crate::v2::handler as waterheater_calc;
use crate::v2::router::v2_routes;

mod common;
mod http;
mod rate_limit;
mod tests;
mod v2;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .json()
        .with_max_level(tracing::Level::INFO)
        .with_current_span(false)
        .with_ansi(false)
        .without_time()
        .with_target(false)
        .init();

    #[derive(OpenApi)]
    #[openapi(
        paths(waterheater_calc::handle_enable_water_heater),
        components(
            schemas(wh_core::types::BiddingZone)
        ),
        tags(
            (name = "waterheater_calc", description = "Easy-to-use API designed to be used with ready-made Shelly scripts for controlling 
                for example a waterheater to be turned on at certain hours of the day.")
        )
    )]
    struct ApiDoc;

    set_var("AWS_LAMBDA_HTTP_IGNORE_STAGE_IN_PATH", "true");

    let app = Router::new()
        .nest("/api/v2", v2_routes())
        .merge(
            SwaggerUi::new("/api/v2/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()),
        )
        .layer(axum::middleware::from_fn(RateLimit::rate_limit));

    run(app).await
}
