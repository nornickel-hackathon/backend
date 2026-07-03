//! Маппинг `UseCaseError` → HTTP-ответ. Единственное место, где application-
//! ошибка получает статус и тело в формате контракта (API_CONVENTIONS.md).

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use contracts::ApiError;
use serde_json::Value;

use crate::application::UseCaseError;

pub struct HttpError(pub UseCaseError);

impl From<UseCaseError> for HttpError {
    fn from(e: UseCaseError) -> Self {
        HttpError(e)
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let (status, body) = match self.0 {
            UseCaseError::Validation(api) => (StatusCode::UNPROCESSABLE_ENTITY, api),
            UseCaseError::NotFound(m) => (
                StatusCode::NOT_FOUND,
                ApiError::new("NOT_FOUND", m, Value::Null),
            ),
            UseCaseError::Internal(m) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiError::new("INTERNAL_ERROR", m, Value::Null),
            ),
        };
        (status, Json(body)).into_response()
    }
}
