use axum::body::{Body, Bytes};
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response, Result};
use std::sync::atomic::{AtomicU64, Ordering};

static REQUEST_NUMBER: AtomicU64 = AtomicU64::new(0);

// From axum/examples/print-request-response
pub async fn print_request_response(
    request: Request<Body>,
    next: Next<Body>,
) -> Result<impl IntoResponse> {
    let request_number = REQUEST_NUMBER.fetch_add(1, Ordering::SeqCst);

    tracing::debug!("<{request_number} {} {}", request.method(), request.uri());
    let (parts, body) = request.into_parts();
    let bytes = buffer_and_print("<", request_number, body).await?;
    let request = Request::from_parts(parts, Body::from(bytes));

    let response = next.run(request).await;

    tracing::debug!(">{request_number} {}", response.status());
    let (parts, body) = response.into_parts();
    let bytes = buffer_and_print(">", request_number, body).await?;
    let response = Response::from_parts(parts, Body::from(bytes));

    Ok(response)
}

async fn buffer_and_print<B>(direction: &str, request_number: u64, body: B) -> Result<Bytes>
where
    B: axum::body::HttpBody<Data = Bytes>,
    B::Error: std::fmt::Display,
{
    let bytes = hyper::body::to_bytes(body).await.map_err(|err| {
        (
            StatusCode::BAD_REQUEST,
            format!("failed to read {direction}{request_number} body: {err}"),
        )
    })?;

    if let Ok(body) = std::str::from_utf8(&bytes) {
        tracing::debug!("{direction}{request_number} body = {:?}", body);
    }

    Ok(bytes)
}
