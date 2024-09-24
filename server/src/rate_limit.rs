use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

use redis::aio::Connection;
use redis::RedisError;
use std::env;
use tracing::{error, info};

use crate::AppState;

pub struct RateLimit;

impl RateLimit {
    pub async fn rate_limit(
        State(state): State<AppState>,
        request: Request<Body>,
        next: Next,
    ) -> Result<Response, StatusCode> {
        let is_production = env::var::<&str>("DEPLOY_ENV").unwrap_or_default() == "production";

        // Do not rate limit on local env
        if !is_production {
            return Ok(next.run(request).await);
        }

        let client_ip = Self::extract_client_ip(&request).unwrap_or("unknown");

        let capacity = 20; // Maximum 20 requests allowed
        let refill_rate_per_millisecond = 20.0 / 60_000.0; // Tokens per millisecond

        let current_time = chrono::Utc::now().timestamp_millis() as f64 / 1000.0;

        let start_time = std::time::Instant::now();
        let mut conn = match state.redis_pool.get_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to connect to Redis: {}", e);
                return Ok(next.run(request).await);
            }
        };
        let conn_acquisition_time = start_time.elapsed();
        info!(
            "Redis connection acquisition time: {:?}",
            conn_acquisition_time
        );

        let allowed = Self::check_rate_limit(
            &mut conn,
            client_ip,
            capacity,
            refill_rate_per_millisecond,
            current_time,
        )
        .await
        .unwrap_or(true);

        if allowed {
            Ok(next.run(request).await)
        } else {
            info!("Rate limit exceeded for client IP: {}", client_ip);
            let mut response = Response::new("Too Many Requests".into());
            *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
            Ok(response)
        }
    }

    fn extract_client_ip(request: &Request<Body>) -> Option<&str> {
        match request.headers().get("X-Forwarded-For") {
            Some(value) => value.to_str().ok(),
            None => None,
        }
    }

    /// Executes the Lua script to perform token bucket rate limiting
    async fn check_rate_limit(
        conn: &mut Connection,
        client_ip: &str,
        capacity: usize,
        refill_rate: f64,
        current_time: f64,
    ) -> Result<bool, RedisError> {
        let key = format!("rate_limit:{}", client_ip);

        let script = r#"
            local key = KEYS[1]
            local capacity = tonumber(ARGV[1])
            local refill_time = tonumber(ARGV[2])
            local current_time = tonumber(ARGV[3])

            local data = redis.call("GET", key)
            local tokens = tonumber(data)
            if tokens == nil then
                tokens = capacity - 1
                redis.call("SETEX", key, refill_time, tokens)
                return 1
            end

            if tokens > 0 then
                tokens = tokens - 1
                redis.call("SET", key, tokens)
                redis.call("EXPIRE", key, refill_time)
                return 1
            else
                return 0
            end
        "#;

        let allowed: i32 = redis::cmd("EVAL")
            .arg(script)
            .arg(1)
            .arg(&key)
            .arg(capacity)
            .arg(refill_rate)
            .arg(current_time)
            .query_async(conn)
            .await?;

        Ok(allowed == 1)
    }
}
