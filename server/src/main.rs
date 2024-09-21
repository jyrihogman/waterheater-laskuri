use std::env::set_var;

use axum::Router;
use lambda_http::{run, tracing, Error};

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::v2::handler as waterheater_calc;
use crate::v2::router::v2_routes;

mod common;
mod http;
mod tests;
mod v1;
mod v2;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();
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

    let app = Router::new().nest("/api/v2", v2_routes()).merge(
        SwaggerUi::new("/api/v2/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()),
    );

    run(app).await
}
