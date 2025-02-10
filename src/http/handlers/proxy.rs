use crate::core::filter::ProxyFilter;
use crate::core::resolver::ProxyResolver;
use crate::core::script::input::InputBuilder;
use crate::http::body::ProxyBody;
use crate::http::handlers::{Context, HttpHandler, HttpResult, Result};
use anyhow::anyhow;
use bytes::Buf;
use futures::future::BoxFuture;
use http::uri::Scheme;
use http::{header, Uri};
use http_body::Body;
use http_body_util::BodyExt;
use hudsucker::hyper::Request;
use log::info;
use std::fmt::Debug;
use std::marker::PhantomData;

#[derive(Debug)]
pub struct ProxyHandler<B, F>
where
    B: BodyExt + Send + Debug + TryFrom<ProxyBody> + 'static,
    B::Data: Send,
    <B as Body>::Error: Debug,
    <B as TryFrom<ProxyBody>>::Error: Debug,
    F: ProxyFilter + Default + Send + Sync + 'static,
{
    phantom_body: PhantomData<B>,
    phantom_filter: PhantomData<F>,
}

impl<B, F> ProxyHandler<B, F>
where
    B: BodyExt + Send + Debug + TryFrom<ProxyBody> + 'static,
    B::Data: Send,
    <B as Body>::Error: Debug,
    <B as TryFrom<ProxyBody>>::Error: Debug,
    F: ProxyFilter + Default + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            phantom_body: PhantomData,
            phantom_filter: PhantomData,
        }
    }
}

impl<B, F> HttpHandler<B> for ProxyHandler<B, F>
where
    B: BodyExt + Send + Debug + TryFrom<ProxyBody> + 'static,
    B::Data: Send,
    <B as Body>::Error: Debug,
    <B as TryFrom<ProxyBody>>::Error: Debug,
    F: ProxyFilter + Default + Send + Sync + 'static,
{
    fn handle_request(
        &self,
        ctx: &Context,
        request: Request<B>,
    ) -> BoxFuture<'static, Result<HttpResult<B>>> {
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
}

unsafe impl<B, F> Sync for ProxyHandler<B, F>
where
    B: BodyExt + Send + Debug + TryFrom<ProxyBody> + 'static,
    B::Data: Send,
    <B as Body>::Error: Debug,
    <B as TryFrom<ProxyBody>>::Error: Debug,
    F: ProxyFilter + Default + Send + Sync + 'static,
{
}
