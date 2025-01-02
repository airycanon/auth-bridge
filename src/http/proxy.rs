use crate::core::filter::ProxyFilter;
use crate::core::resolver::ProxyResolver;
use crate::core::script::input::InputBuilder;
use crate::http::body::ProxyBody;
use crate::http::{Context, HttpResult, Result};
use anyhow::anyhow;
use bytes::Buf;
use futures::future::BoxFuture;
use http::{header, Uri};
use http_body_util::BodyExt;
use hudsucker::hyper::Request;
use hyper::body::Body;
use log::info;
use std::fmt::Debug;
use http::uri::Scheme;

pub fn proxy_request<'a, B, F>(
    ctx: &Context,
    request: Request<B>,
) -> BoxFuture<'a, Result<HttpResult<B>>>
where
    B: Body + Send + Debug + TryFrom<ProxyBody> + 'a,
    B::Data: Send,
    <B as Body>::Error: Debug,
    <B as TryFrom<ProxyBody>>::Error: Debug,
    F: ProxyFilter + Default + Send + Sync,
{
    let addr = ctx.addr;
    Box::pin(async move {

        let (mut parts, body) = request.into_parts();

        let authority = if let Some(auth) = parts.uri.authority() {
            auth.as_str()
        } else {
            parts
                .headers
                .get(header::HOST)
                .and_then(|h| h.to_str().ok())
                .unwrap_or_default()
        };

        let builder = Uri::builder()
            .scheme(parts.uri.scheme().cloned().unwrap_or(Scheme::HTTP))
            .authority(authority)
            .path_and_query(parts.uri.path());
        parts.uri = builder.build()?;

        info!("request url: {}", parts.uri.to_string());

        let bytes = match body.collect().await {
            Ok(collected) => {
                let mut buf = collected.aggregate();
                buf.copy_to_bytes(buf.remaining())
            }
            Err(e) => return Err(anyhow!("Failed to collect body: {:?}", e)),
        };

        let input = InputBuilder::default()
            .with_parts(parts.clone())
            .with_body(bytes.clone())
            .with_pod_ip(addr.ip())
            .build()?;

        let resolver = ProxyResolver::from_uri(parts.uri.clone(), F::default()).await?;
        let result = resolver.evaluate(&input)?;

        let proxy_body = if result {
            resolver.apply(&mut parts, bytes.clone()).await?
        } else {
            ProxyBody::from(bytes.clone())
        };

        let body = match B::try_from(proxy_body) {
            Ok(body) => body,
            Err(e) => return Err(anyhow!("Failed to convert body: {:?}", e)),
        };

        Ok(HttpResult::Request(Request::from_parts(parts, body)))
    })
}
