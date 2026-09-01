//! Request-body handling for the Binary Ninja WARP client.
//!
//! The plugin's `NetworkClient` puts `Content-Encoding: gzip` on **every**
//! request, but the download providers it uses send the JSON body as-is.
//! actix's `Json`/`Form`/`Payload` extractors trust that header and would
//! reject the plain body with a 400, so this middleware buffers the body,
//! decodes it only when it really starts with the gzip magic, and hands the
//! request on without the header.

use std::{
    future::{Future, Ready, ready},
    io::Read,
    pin::Pin,
    rc::Rc,
};

use actix_web::{
    Error, HttpMessage,
    body::MessageBody,
    dev::{Payload, Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
    error::{ErrorBadRequest, ErrorPayloadTooLarge},
    http::header::{CONTENT_ENCODING, CONTENT_LENGTH, HeaderValue},
    web::{Bytes, BytesMut},
};
use futures_util::StreamExt;

/// Largest request body we are willing to buffer (matches the JSON limit).
pub const MAX_BODY_BYTES: usize = 96 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default)]
pub struct SniffGzipBody;

impl<S, B> Transform<S, ServiceRequest> for SniffGzipBody
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = SniffGzipBodyMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(SniffGzipBodyMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct SniffGzipBodyMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for SniffGzipBodyMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let service = Rc::clone(&self.service);
        Box::pin(async move {
            if claims_gzip(&req) {
                let raw = read_payload(&mut req).await?;
                let body = if is_gzip(&raw) { gunzip(&raw)? } else { raw };
                let headers = req.headers_mut();
                headers.remove(CONTENT_ENCODING);
                headers.insert(CONTENT_LENGTH, HeaderValue::from(body.len()));
                req.set_payload(Payload::from(body));
            }
            service.call(req).await
        })
    }
}

fn claims_gzip(req: &ServiceRequest) -> bool {
    req.headers()
        .get(CONTENT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim().eq_ignore_ascii_case("gzip"))
        .unwrap_or(false)
}

fn is_gzip(bytes: &[u8]) -> bool {
    bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b
}

async fn read_payload(req: &mut ServiceRequest) -> Result<Bytes, Error> {
    let mut payload = Box::pin(req.take_payload());
    let mut buf = BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        if buf.len() + chunk.len() > MAX_BODY_BYTES {
            return Err(ErrorPayloadTooLarge("request body too large"));
        }
        buf.extend_from_slice(&chunk);
    }
    Ok(buf.freeze())
}

fn gunzip(bytes: &[u8]) -> Result<Bytes, Error> {
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(bytes)
        .take(MAX_BODY_BYTES as u64 + 1)
        .read_to_end(&mut out)
        .map_err(|e| ErrorBadRequest(format!("invalid gzip request body: {e}")))?;
    if out.len() > MAX_BODY_BYTES {
        return Err(ErrorPayloadTooLarge("request body too large"));
    }
    Ok(Bytes::from(out))
}
