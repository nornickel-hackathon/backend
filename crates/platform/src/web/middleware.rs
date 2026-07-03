//! HTTP-middleware платформы.

use axum::extract::Request;
use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::Response;

use crate::CONTRACT_VERSION;

/// Проставляет заголовок версии контракта на все ответы (API_CONVENTIONS.md).
pub async fn contract_version_header(req: Request, next: Next) -> Response {
    let mut res = next.run(req).await;
    res.headers_mut().insert(
        "X-Contract-Version",
        HeaderValue::from_static(CONTRACT_VERSION),
    );
    res
}
