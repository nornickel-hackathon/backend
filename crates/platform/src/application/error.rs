//! HTTP-агностичная ошибка use case. Web-слой — единственное место, где она
//! превращается в конкретный статус и тело ответа.

use contracts::ApiError;

pub enum UseCaseError {
    /// Невалидный вход или граф — готовое тело контракта (web → 422).
    Validation(ApiError),
    /// Ресурс не найден (web → 404).
    NotFound(String),
    /// Внутренний сбой: I/O, парсинг фикстур и т.п. (web → 500).
    Internal(String),
}
