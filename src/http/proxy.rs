use crate::core::filter::AddressFilter;
use crate::core::resolver::ProxyResolver;
use crate::core::script::input::InputBuilder;
use crate::http::body::ProxyBody;
use crate::http::{Context, HttpResult, Result};
use anyhow::anyhow;
use bytes::Buf;
use futures::future::BoxFuture;
use http_body_util::BodyExt;
use hudsucker::hyper::Request;
use hyper::body::Body;
use log::info;
use std::fmt::Debug;

pub fn proxy_request<B>(
    ctx: &Context,
    request: Request<B>,
) -> BoxFuture<'static, Result<HttpResult<B>>>
where
    B: Body + Send + Debug + TryFrom<ProxyBody> + 'static,
    B::Data: Send,
    <B as Body>::Error: Debug,
    <B as TryFrom<ProxyBody>>::Error: Debug,
{
    let addr = ctx.addr;
    Box::pin(async move {
        info!("request url: {}", request.uri().to_string());

        let (mut parts, body) = request.into_parts();

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

        let resolver = ProxyResolver::from_uri(parts.uri.clone(), AddressFilter).await?;
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
