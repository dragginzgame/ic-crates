//! Cargo API token gate and sparse-index projection; no provider request proxy.

use crate::{Error, Result, registry::Registry, upload, workflow::Publisher};
use ic_crates_protocol::HttpResponse;
use subtle::ConstantTimeEq;
use tiny_http::{Header, Method, Request, Response, StatusCode};

pub(crate) fn handle<B: Registry>(mut request: Request, publisher: &Publisher<B>, token: &str) {
    let head = request.method() == &Method::Head;
    let result = route(&mut request, publisher, token);
    let mut response = match result {
        Ok(response) => response,
        Err(error) => json(
            error.status,
            serde_json::json!({"errors":[{"detail":error.detail}]}),
        ),
    };
    if head {
        response.body.clear();
    }
    let mut outgoing =
        Response::from_data(response.body).with_status_code(StatusCode(response.status_code));
    for (name, value) in response.headers {
        if let Ok(header) = Header::from_bytes(name, value) {
            outgoing.add_header(header);
        }
    }
    let _ = request.respond(outgoing);
}

pub(crate) fn json(status_code: u16, value: serde_json::Value) -> HttpResponse {
    HttpResponse {
        status_code,
        headers: vec![
            ("Content-Type".into(), "application/json".into()),
            ("Cache-Control".into(), "no-store".into()),
        ],
        body: value.to_string().into_bytes(),
        upgrade: None,
    }
}

fn route<B: Registry>(
    request: &mut Request,
    publisher: &Publisher<B>,
    token: &str,
) -> Result<HttpResponse> {
    let path = request.url().split('?').next().unwrap_or("").to_owned();
    if request.url().len() > 2048 {
        return Err(Error::bad("Request URL exceeds limit."));
    }
    if request.method() == &Method::Put && path == "/api/v1/crates/new" {
        let auth: Vec<_> = request
            .headers()
            .iter()
            .filter(|header| header.field.equiv("Authorization"))
            .collect();
        if auth.len() != 1 || !bool::from(auth[0].value.as_str().as_bytes().ct_eq(token.as_bytes()))
        {
            return Err(Error {
                status: 403,
                detail: "Invalid registry token.",
            });
        }
        if !request.headers().iter().any(|header| {
            header.field.equiv("Content-Type")
                && header.value.as_str() == "application/octet-stream"
        }) {
            return Err(Error::bad("Expected application/octet-stream."));
        }
        if request.body_length().is_some_and(|length| {
            length as u64 > publisher.config.max_archive_bytes + 256 * 1024 + 8
        }) {
            return Err(Error {
                status: 413,
                detail: "Upload exceeds configured archive/metadata limits.",
            });
        }
        let index = format!("sparse+{}/index/", publisher.config.public_origin);
        let upload = upload::receive(
            &mut request.as_reader(),
            publisher.config.max_archive_bytes,
            &publisher.config.spool,
            &index,
        )?;
        publisher.publish(upload)?;
        return Ok(json(
            200,
            serde_json::json!({"warnings":{"invalid_categories":[],"invalid_badges":[],"other":[]}}),
        ));
    }
    if !matches!(request.method(), Method::Get | Method::Head) {
        return Err(Error {
            status: 405,
            detail: "Method not supported.",
        });
    }
    if request.body_length().is_some_and(|length| length != 0) {
        return Err(Error::bad("Read requests cannot contain a body."));
    }
    if path == "/index/config.json" {
        // Verify the configured IC catalog is serving before advertising it.
        let response = publisher.registry.read(&path)?;
        if response.status_code != 200 {
            return Ok(response);
        }
        let origin = &publisher.config.public_origin;
        return Ok(json(
            200,
            serde_json::json!({"api":origin,"dl":format!("{origin}/api/v1/crates/{{crate}}/{{version}}/download")}),
        ));
    }
    if path.starts_with("/index/") || path.starts_with("/api/v1/crates/") || path == "/health" {
        let response = publisher.registry.read(&path)?;
        if response.body.len() > 1024 * 1024
            || response.headers.len() > 32
            || response.upgrade == Some(true)
        {
            return Err(Error::unavailable("Invalid catalog HTTP reply."));
        }
        return Ok(response);
    }
    Err(Error {
        status: 404,
        detail: "Endpoint not found.",
    })
}
