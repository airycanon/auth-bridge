use crate::core::error::ApiError;
use crate::core::error::ParseError;
use crate::core::filter::AddressFilter;
use crate::core::resolver::ProxyResolver;
use crate::core::script::input::InputBuilder;
use crate::proxy::forward::multi::{HttpHandler, ProxyResult};
use http_body_util::BodyExt;
use http_body_util::Full;
use hudsucker::hyper::Request;
use hudsucker::tokio_tungstenite::tungstenite::http::Method;
use hudsucker::{Body, HttpContext, RequestOrResponse};
use log::info;

#[derive(Clone, Default)]
pub struct ProxyHandler;

impl HttpHandler for ProxyHandler {
    fn handle_request<'a>(&'a self, ctx: &'a HttpContext, request: Request<Body>) -> ProxyResult {
        Box::pin(async move {
            if request.method() == Method::CONNECT {
                return RequestOrResponse::Request(request);
            }

            info!("request url: {}", request.uri().to_string());

            let (parts, body) = request.into_parts();

            let bytes = match body.collect().await {
                Ok(input) => input.to_bytes(),
                Err(err) => return ParseError(err).into(),
            };
            let new_body = Body::from(Full::from(bytes.clone()));

            let input = match InputBuilder::default()
                .with_uri(parts.uri.clone())
                .with_body(parts.clone(), bytes.clone())
                .with_pod_ip(ctx.client_addr.ip())
                .build()
            {
                Ok(input) => input,
                Err(err) => return ParseError(err).into(),
            };

            let resolver = match ProxyResolver::from_uri(parts.uri.clone(), AddressFilter).await {
                Ok(resolver) => resolver,
                Err(err) => return ApiError(err).into(),
            };

            let result = match resolver.evaluate(&input) {
                Ok(result) => result,
                Err(err) => return ParseError(err).into(),
            };
            if !result {
                return RequestOrResponse::Request(Request::from_parts(parts, new_body));
            }

            let (new_parts, new_body) = match resolver.apply(parts.clone(), bytes.clone()).await {
                Ok(parts_body) => parts_body,
                Err(err) => return ParseError(err).into(),
            };

            let body = match new_body.try_into() {
                Ok(body) => body,
                Err(err) => return ParseError(err).into(),
            };

            RequestOrResponse::Request(Request::from_parts(new_parts, body))
        })
    }
}
