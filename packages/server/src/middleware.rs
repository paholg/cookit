//! Cross-cutting axum middleware.

use axum::body::{Body, to_bytes};
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

/// Logs any 5xx response with the request method, URI, status, and body.
///
/// Server-function failures are turned into bare HTTP error responses by the
/// dioxus runtime — without this layer there's no record on the server side
/// that anything went wrong, and the client gets a status code with nothing
/// to act on. Apply this to the merged app router so it covers both auth and
/// server-function routes.
pub async fn log_server_errors(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();

    let resp = next.run(req).await;
    if !resp.status().is_server_error() {
        return resp;
    }

    let (parts, body) = resp.into_parts();
    // 64 KiB is plenty for an error body; anything bigger gets truncated in
    // the log rather than blowing up memory.
    let bytes = to_bytes(body, 64 * 1024).await.unwrap_or_default();
    let body_text = String::from_utf8_lossy(&bytes);
    tracing::error!(
        %method,
        %uri,
        status = %parts.status,
        body = %body_text,
        "5xx response",
    );

    Response::from_parts(parts, Body::from(bytes))
}
