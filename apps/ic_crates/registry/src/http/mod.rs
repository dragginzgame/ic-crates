//! Cargo HTTP reads. Initial implementation uses the gateway's update upgrade.

use candid::CandidType;
use ic_crates_protocol::{RegistryError, index_path};
use serde::Deserialize;

use crate::ops;

#[derive(CandidType, Deserialize)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub certificate_version: Option<u16>,
}

#[derive(CandidType, Deserialize)]
pub struct HttpResponse {
    pub status_code: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub upgrade: Option<bool>,
}

impl HttpResponse {
    pub(crate) fn upgrade() -> Self {
        Self {
            status_code: 200,
            headers: Vec::new(),
            body: Vec::new(),
            upgrade: Some(true),
        }
    }

    fn response(status: u16, content_type: &str, body: Vec<u8>) -> Self {
        Self {
            status_code: status,
            headers: vec![
                ("Content-Type".into(), content_type.into()),
                ("Cache-Control".into(), "no-store".into()),
            ],
            body,
            upgrade: None,
        }
    }

    fn error(status: u16, message: &str) -> Self {
        Self::response(
            status,
            "application/json",
            serde_json::json!({"errors":[{"detail":message}]})
                .to_string()
                .into_bytes(),
        )
    }
}

pub(crate) fn serve(request: &HttpRequest) -> HttpResponse {
    let mut response = if !matches!(request.method.as_str(), "GET" | "HEAD") {
        HttpResponse::error(
            405,
            "This bootstrap registry accepts publication through authenticated Candid.",
        )
    } else if request.url.len() > 2048 || !request.body.is_empty() {
        HttpResponse::error(400, "Invalid read request.")
    } else {
        match route(request.url.split('?').next().unwrap_or("")) {
            Ok(response) => response,
            Err(
                RegistryError::NotFound | RegistryError::InvalidName | RegistryError::NameConflict,
            ) => HttpResponse::error(404, "Crate or version not found."),
            Err(_) => HttpResponse::error(503, "Registry catalog unavailable."),
        }
    };
    if request.method == "HEAD" {
        response.body.clear();
    }
    response
}

fn route(path: &str) -> Result<HttpResponse, RegistryError> {
    match path {
        "/health" => {
            return Ok(HttpResponse::response(
                200,
                "application/json",
                b"{\"status\":\"bootstrap\"}".to_vec(),
            ));
        }
        "/index/config.json" => {
            ops::configuration()?;
            let base = format!("https://{}.icp0.io", ic_cdk::api::canister_self());
            // Omit `api`: Cargo publish/login are intentionally not advertised yet.
            let config = serde_json::json!({"dl":format!("{base}/api/v1/crates/{{crate}}/{{version}}/download")});
            return Ok(HttpResponse::response(
                200,
                "application/json",
                config.to_string().into_bytes(),
            ));
        }
        _ => {}
    }
    if let Some(index) = path.strip_prefix("/index/") {
        let name = index.rsplit('/').next().ok_or(RegistryError::NotFound)?;
        if index_path(name)? != index {
            return Err(RegistryError::NotFound);
        }
        return Ok(HttpResponse::response(
            200,
            "text/plain; charset=utf-8",
            ops::index(name)?.into_bytes(),
        ));
    }
    if let Some(download) = path.strip_prefix("/api/v1/crates/") {
        let fields: Vec<_> = download.split('/').collect();
        if let [name, version, "download"] = fields.as_slice() {
            let row = ops::version(name, version)?;
            let mut response = HttpResponse::response(302, "application/octet-stream", Vec::new());
            response.headers.push(("Location".into(), row.download_url));
            return Ok(response);
        }
    }
    Err(RegistryError::NotFound)
}
