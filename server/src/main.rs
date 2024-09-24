use std::env::set_var;

use axum::Router;
use lambda_http::tower::ServiceBuilder;
use lambda_http::{run, Error};

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::v2::handler as waterheater_calc;
use crate::v2::router::v2_routes;

mod common;
mod http;
mod middleware;
mod rate_limiter;
mod tests;
mod v2;

#[derive(Clone)]
struct AppState {
    dynamo_client: aws_sdk_dynamodb::Client,
}

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

    let config = aws_config::load_from_env().await;
    let client = aws_sdk_dynamodb::Client::new(&config);

    let state = AppState {
        dynamo_client: client,
    };

    set_var("AWS_LAMBDA_HTTP_IGNORE_STAGE_IN_PATH", "true");

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

    let app = Router::new()
        .nest("/api/v2", v2_routes())
        .merge(
            SwaggerUi::new("/api/v2/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()),
        )
        .layer(
            ServiceBuilder::new()
                .layer(axum::middleware::from_fn(middleware::inject_connect_info))
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    rate_limiter::rate_limit,
                )),
        )
        .with_state(state);

    run(app).await
}
