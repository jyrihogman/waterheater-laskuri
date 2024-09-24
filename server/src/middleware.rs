use axum::body::Body;
use axum::{
    extract::connect_info::ConnectInfo, http::Request, middleware::Next, response::Response,
};
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

pub async fn inject_connect_info(mut req: Request<Body>, next: Next) -> Response {
    // Extract the client's IP address from the `X-Forwarded-For` header
    // Defaults to localhost if not found
    let client_ip = req
        .headers()
        .get("X-Forwarded-For")
        .and_then(|value| value.to_str().ok())
        .and_then(|s| s.split(',').next())
        .and_then(|s| IpAddr::from_str(s.trim()).ok())
        .unwrap_or_else(|| IpAddr::from([127, 0, 0, 1]));

    let socket_addr = SocketAddr::new(client_ip, 0);

    req.extensions_mut().insert(ConnectInfo(socket_addr));

    next.run(req).await
}
