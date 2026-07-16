use std::time::Instant;

use axum::{extract::Request, middleware::Next, response::Response};

pub async fn trace_request(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = Instant::now();

    let response = next.run(req).await;

    let latency = start.elapsed();
    let status = response.status();

    tracing::info!(
        method = %method,
        uri = %uri,
        status = %status.as_u16(),
        latency = ?latency,
        "HTTP request completed"
    );

    response
}
