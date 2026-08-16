//! Cross-cutting axum middleware.

use {
    axum::{
        body::{Body, to_bytes},
        extract::Request,
        middleware::Next,
        response::Response,
    },
    tracing::{Instrument, field::Empty},
};

/// Opens a span per request, which is what gets exported to the collector.
///
/// The `otel.*` fields are the ones `tracing-opentelemetry` maps onto the
/// OpenTelemetry span itself rather than treating as attributes. Apply this
/// outside [`log_server_errors`] so its 5xx events land inside the span.
pub async fn trace_requests(req: Request, next: Next) -> Response {
    let path = req.uri().path();
    // Static assets and the devtools socket would drown out real requests.
    if path.starts_with("/assets/") || path.starts_with("/_dioxus") {
        return next.run(req).await;
    }

    let span = tracing::info_span!(
        "http_request",
        otel.name = format!("{} {}", req.method(), path),
        otel.kind = "server",
        otel.status_code = Empty,
        http.request.method = %req.method(),
        url.path = %path,
        http.response.status_code = Empty,
    );

    let resp = next.run(req).instrument(span.clone()).await;

    span.record("http.response.status_code", resp.status().as_u16());
    if resp.status().is_server_error() {
        span.record("otel.status_code", "ERROR");
    }

    resp
}

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
