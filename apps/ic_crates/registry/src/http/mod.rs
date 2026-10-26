//! Cargo HTTP reads. Initial implementation uses the gateway's update upgrade.

pub(crate) mod codec;

pub use ic_crates_protocol::{HttpRequest, HttpResponse};
use ic_crates_protocol::{RegistryError, index_path};

use crate::ops;

pub(crate) fn upgrade() -> HttpResponse {
    HttpResponse {
        status_code: 200,
        headers: Vec::new(),
        body: Vec::new(),
        upgrade: Some(true),
    }
}

fn response(status: u16, content_type: &str, body: Vec<u8>) -> HttpResponse {
    HttpResponse {
        status_code: status,
        headers: vec![
            ("Content-Type".into(), content_type.into()),
            ("Cache-Control".into(), "no-store".into()),
        ],
        body,
        upgrade: None,
    }
}

fn error(status: u16, message: &str) -> HttpResponse {
    response(
        status,
        "application/json",
        serde_json::json!({"errors":[{"detail":message}]})
            .to_string()
            .into_bytes(),
    )
}

pub(crate) fn serve(request: &HttpRequest) -> HttpResponse {
    let mut response = if !matches!(request.method.as_str(), "GET" | "HEAD") {
        error(
            405,
            "This bootstrap registry accepts publication through authenticated Candid.",
        )
    } else if request.url.len() > 2048 || !request.body.is_empty() {
        error(400, "Invalid read request.")
    } else {
        match route(request.url.split('?').next().unwrap_or("")) {
            Ok(response) => response,
            Err(
                RegistryError::NotFound | RegistryError::InvalidName | RegistryError::NameConflict,
            ) => error(404, "Crate or version not found."),
            Err(_) => error(503, "Registry catalog unavailable."),
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
            return Ok(response(
                200,
                "application/json",
                b"{\"status\":\"bootstrap\"}".to_vec(),
            ));
        }
        "/index/config.json" => {
            ops::configuration()?;
            let base = format!("https://{}.icp0.io", ic_platform::cdk::api::canister_self());
            // Direct canister reads omit `api`; the host bridge advertises its own API.
            let config = serde_json::json!({"dl":format!("{base}/api/v1/crates/{{crate}}/{{version}}/download")});
            return Ok(response(
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
        return Ok(response(
            200,
            "text/plain; charset=utf-8",
            ops::index(name)?.into_bytes(),
        ));
    }
    if let Some(download) = path.strip_prefix("/api/v1/crates/") {
        let fields: Vec<_> = download.split('/').collect();
        if let [name, version, "download"] = fields.as_slice() {
            let row = ops::version(name, version)?;
            let mut response = response(302, "application/octet-stream", Vec::new());
            response.headers.push(("Location".into(), row.download_url));
            return Ok(response);
        }
    }
    Err(RegistryError::NotFound)
}
