use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

use deadpool_redis::Connection;
use redis::RedisError;

use std::{
    env,
    net::{IpAddr, SocketAddr},
};
use tracing::{error, info};

use crate::AppState;

pub async fn rate_limit(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let is_production = env::var::<&str>("DEPLOY_ENV").unwrap_or_default() == "production";

    // Do not rate limit on local env
    if !is_production {
        return Ok(next.run(request).await);
    }

    let client_ip = addr.ip();

    // Maximum 20 requests allowed
    let capacity = 20;

    // Refill rate: 20 tokens per minute
    let refill_rate = 20.0 / 60.0;

    let current_time = chrono::Utc::now().timestamp_millis() as f64 / 1000.0;

    let mut conn = match state.redis_pool.get().await {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to connect to Redis: {}", e);
            // Allow the request if Redis is unavailable
            return Ok(next.run(request).await);
        }
    };

    let allowed =
        match check_rate_limit(&mut conn, client_ip, capacity, refill_rate, current_time).await {
            Ok(val) => val,
            Err(e) => {
                error!("Rate limit check failed: {}", e);
                true
            }
        };

    if allowed {
        Ok(next.run(request).await)
    } else {
        info!("Rate limit exceeded for client IP: {}", client_ip);
        let mut response = Response::new("Too Many Requests".into());
        *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
        Ok(response)
    }
}

// Executes the Lua script to perform token bucket rate limiting
async fn check_rate_limit(
    conn: &mut Connection,
    client_ip: IpAddr,
    capacity: usize,
    refill_rate: f64,
    current_time: f64,
) -> Result<bool, RedisError> {
    let key = format!("rate_limit:{}", client_ip);

    let script = r#"
            local key = KEYS[1]
            local capacity = tonumber(ARGV[1])
            local refill_rate = tonumber(ARGV[2])
            local current_time = tonumber(ARGV[3])

            local data = redis.call("HMGET", key, "tokens", "last_refill")
            local tokens = tonumber(data[1])
            local last_refill = tonumber(data[2])

            if tokens == nil then
                tokens = capacity
                last_refill = current_time
            end

            local delta = current_time - last_refill
            local tokens_to_add = delta * refill_rate
            tokens = math.min(tokens + tokens_to_add, capacity)
            last_refill = current_time

            local allowed = 0
            if tokens >= 1 then
                allowed = 1
                tokens = tokens - 1
            end

            redis.call("HMSET", key, "tokens", tokens, "last_refill", last_refill)
            redis.call("EXPIRE", key, 3600)

            return allowed
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
