//! HTTP-адаптеры к Python-сайдкару (2.1-бис). Уход от моков: `/diagnose` и
//! `/extract` берутся у живого сайдкара по `SIDECAR_URL`, а при любой ошибке/
//! недоступности — детерминированный fallback на файловые фикстуры (демо-
//! страховка сквозная). Схемы ответов контрактные — сайдкар отдаёт их байт-в-байт.

use std::time::Duration;

use contracts::{DiagnosticsReport, ExtractResponse};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

use crate::application::ports::{DiagnosticsSource, ExtractSource};
use crate::infrastructure::{FileDiagnosticsSource, FileExtractSource};

/// Выполнить blocking-POST на отдельном std-потоке: reqwest::blocking держит
/// собственный runtime, который нельзя ронять внутри async-контекста tokio.
fn blocking_post<T: DeserializeOwned + Send + 'static>(
    url: String,
    body: Value,
) -> Result<T, String> {
    std::thread::spawn(move || -> Result<T, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_millis(1500))
            .build()
            .map_err(|e| e.to_string())?;
        client
            .post(&url)
            .json(&body)
            .send()
            .and_then(|r| r.error_for_status())
            .map_err(|e| e.to_string())?
            .json::<T>()
            .map_err(|e| e.to_string())
    })
    .join()
    .map_err(|_| "sidecar request thread panicked".to_string())?
}

fn mime_of(path: &str) -> &'static str {
    if path.ends_with(".pdf") {
        "application/pdf"
    } else if path.ends_with(".csv") {
        "text/csv"
    } else if path.ends_with(".docx") {
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    } else {
        "text/plain"
    }
}

/// `DiagnosticsSource` через `POST {SIDECAR_URL}/diagnose`, fallback на файл.
pub struct HttpDiagnosticsSource {
    sidecar_url: String,
    fallback: FileDiagnosticsSource,
}

impl HttpDiagnosticsSource {
    pub fn new(sidecar_url: String, fallback: FileDiagnosticsSource) -> Self {
        HttpDiagnosticsSource {
            sidecar_url,
            fallback,
        }
    }
}

impl DiagnosticsSource for HttpDiagnosticsSource {
    fn load(
        &self,
        factory_id: &str,
        source_file: Option<&str>,
        pack_id: &str,
    ) -> Result<DiagnosticsReport, String> {
        let url = format!("{}/diagnose", self.sidecar_url.trim_end_matches('/'));
        if let Some(file_path) = source_file {
            let body = json!({
                "factory_id": factory_id,
                "file_path": file_path,
                "pack_id": pack_id,
            });
            return blocking_post::<DiagnosticsReport>(url, body);
        }

        // For known factories, keep the file version as source of file_path and
        // deterministic demo fallback.
        let file = self.fallback.load(factory_id, None, pack_id)?;
        let body = json!({
            "factory_id": factory_id,
            "file_path": file.source_file,
            "pack_id": if file.pack_id.is_empty() { pack_id } else { &file.pack_id },
        });
        match blocking_post::<DiagnosticsReport>(url, body) {
            Ok(report) => Ok(report),
            Err(e) => {
                eprintln!("sidecar /diagnose failed for '{factory_id}': {e}; using file fallback");
                Ok(file)
            }
        }
    }
}

/// `ExtractSource` через `POST {SIDECAR_URL}/extract`, fallback на файл.
pub struct HttpExtractSource {
    sidecar_url: String,
    fallback: FileExtractSource,
}

impl HttpExtractSource {
    pub fn new(sidecar_url: String, fallback: FileExtractSource) -> Self {
        HttpExtractSource {
            sidecar_url,
            fallback,
        }
    }
}

impl ExtractSource for HttpExtractSource {
    fn load(&self) -> Result<ExtractResponse, String> {
        let file = self.fallback.load()?;
        let url = format!("{}/extract", self.sidecar_url.trim_end_matches('/'));
        let docs: Vec<Value> = file
            .documents
            .iter()
            .map(|d| json!({ "path": d.path, "mime": mime_of(&d.path) }))
            .collect();
        let body = json!({ "docs": docs, "pack_id": file.pack_id });
        match blocking_post::<ExtractResponse>(url, body) {
            Ok(extract) => Ok(extract),
            Err(e) => {
                eprintln!("sidecar /extract failed: {e}; using file fallback");
                Ok(file)
            }
        }
    }
}
