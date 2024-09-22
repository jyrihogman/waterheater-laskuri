use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use once_cell::sync::OnceCell;
use redis::{Client, RedisError};
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;
use tracing::{error, info};

static REDIS_CONNECTION: OnceCell<Arc<AsyncMutex<redis::aio::Connection>>> = OnceCell::new();

pub struct RateLimit;

impl RateLimit {
    // Initializes and retrieves the Redis connection.
    async fn get_redis_connection() -> Result<Arc<AsyncMutex<redis::aio::Connection>>, RedisError> {
        if let Some(conn) = REDIS_CONNECTION.get() {
            return Ok(conn.clone());
        }

        let redis_endpoint = std::env::var("REDIS_ENDPOINTS").expect("REDIS_ENDPOINTS must be set");
        let redis_url = format!("rediss://{}", redis_endpoint);

        println!("Redis URL: {}", redis_url);
        info!("Redis URL: {}", redis_url);

        let client = Client::open(redis_url)?;
        let connection = client.get_async_connection().await?;
        let arc_conn = Arc::new(AsyncMutex::new(connection));

        REDIS_CONNECTION.set(arc_conn.clone()).map_err(|_| {
            RedisError::from((redis::ErrorKind::IoError, "Connection already initialized"))
        })?;

        info!("Redis connection established and initialized.");

        Ok(arc_conn)
    }

    pub async fn rate_limit(request: Request<Body>, next: Next) -> Result<Response, StatusCode> {
        let client_ip = Self::extract_client_ip(&request).unwrap_or("unknown");

        let capacity = 20; // Maximum 20 requests allowed
        let refill_rate = 20.0 / 60.0; // Refill rate: 20 tokens per minute

        let current_time = chrono::Utc::now().timestamp_millis() as f64 / 1000.0;

        let conn = match Self::get_redis_connection().await {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to connect to Redis: {}", e);
                // Allow the request if Redis is unavailable
                return Ok(next.run(request).await);
            }
        };

        let mut lock = conn.lock().await;

        let allowed =
            match Self::check_rate_limit(&mut lock, client_ip, capacity, refill_rate, current_time)
                .await
            {
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

    fn extract_client_ip(request: &Request<Body>) -> Option<&str> {
        match request.headers().get("X-Forwarded-For") {
            Some(value) => value.to_str().ok(),
            None => None,
        }
    }

    /// Executes the Lua script to perform token bucket rate limiting
    async fn check_rate_limit(
        conn: &mut redis::aio::Connection,
        client_ip: &str,
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
}
