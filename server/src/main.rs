use std::env::{self, set_var};
use std::sync::Arc;

use axum::Router;
use lambda_http::tower::ServiceBuilder;
use lambda_http::{run, Error};

use deadpool::managed::{PoolConfig, QueueMode, Timeouts};
use deadpool_redis::{Config, Pool, Runtime};

use lazy_static::lazy_static;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::rate_limit::rate_limit;
use crate::v2::handler as waterheater_calc;
use crate::v2::router::v2_routes;

mod common;
mod http;
mod middleware;
mod rate_limit;
mod tests;
mod v2;

lazy_static! {
    static ref REDIS_POOL: Arc<Pool> = Arc::new(create_redis_pool());
}

#[derive(Clone)]
struct AppState {
    pub redis_pool: Arc<Pool>,
    dynamo_client: aws_sdk_dynamodb::Client,
}

fn create_redis_pool() -> Pool {
    let redis_endpoint = env::var("REDIS_ENDPOINT").unwrap_or("http://localhost".into());
    let redis_url = format!("redis://{}", redis_endpoint);

    let cfg = Config {
        connection: None,
        url: Some(redis_url),
        pool: Some(PoolConfig {
            max_size: 10,
            timeouts: Timeouts::default(),
            queue_mode: QueueMode::Fifo,
        }),
    };

    cfg.create_pool(Some(Runtime::Tokio1)).unwrap()
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    set_var("AWS_LAMBDA_HTTP_IGNORE_STAGE_IN_PATH", "true");
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
        redis_pool: REDIS_POOL.clone(),
        dynamo_client: client,
    };

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
                    rate_limit,
                )),
        )
        .with_state(state);

    run(app).await
}
