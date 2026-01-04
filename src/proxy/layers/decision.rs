use crate::runtime::support::filter::ProxyFilter;
use crate::runtime::support::resolver::ProxyResolver;
use crate::runtime::script::input::InputBuilder;
use bytes::{Buf, Bytes};
use http_body_util::BodyExt;
use rama::{
    Layer, Service,
    http::{Body, Request, Response, StatusCode},
    http::service::web::response::IntoResponse,
    net::stream::SocketInfo,
    telemetry::tracing,
};
use std::{convert::Infallible, net::SocketAddr, sync::Arc};

#[derive(Clone, Debug)]
pub struct ProxyDecision {
    pub(crate) resolver: Arc<ProxyResolver>,
    pub(crate) should_apply: bool,
    pub(crate) bytes: Bytes,
}

#[derive(Clone, Debug)]
pub struct DecisionLayer<F> {
    filter: F,
}

impl<F> DecisionLayer<F> {
    pub fn new(filter: F) -> Self {
        Self { filter }
    }
}

impl<F> Default for DecisionLayer<F>
where
    F: Default,
{
    fn default() -> Self {
        Self {
            filter: F::default(),
        }
    }
}

impl<S, F> Layer<S> for DecisionLayer<F>
where
    F: ProxyFilter + Clone + Send + Sync + 'static,
{
    type Service = DecisionService<S, F>;

    fn layer(&self, inner: S) -> Self::Service {
        DecisionService {
            inner,
            filter: self.filter.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct DecisionService<S, F> {
    inner: S,
    filter: F,
}

impl<S, F> Service<Request> for DecisionService<S, F>
where
    S: Service<Request, Output = Response, Error = Infallible>,
    F: ProxyFilter + Clone + Send + Sync + 'static,
{
    type Output = Response;
    type Error = Infallible;

    async fn serve(&self, req: Request) -> Result<Self::Output, Self::Error> {
        let (parts, body) = req.into_parts();
        let addr = parts
            .extensions
            .get::<SocketInfo>()
            .map(|info| *info.peer_addr())
            .unwrap_or_else(|| SocketAddr::from(([0, 0, 0, 0], 0)));

        let bytes = match body.collect().await {
            Ok(collected) => {
                let mut buf = collected.aggregate();
                buf.copy_to_bytes(buf.remaining())
            }
            Err(err) => {
                tracing::error!(error = ?err, "failed to collect body");
                return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response());
            }
        };

        let input = match InputBuilder::default()
            .with_parts_ref(&parts)
            .with_body_ref(&bytes)
            .with_pod_ip(addr.ip())
            .build()
        {
            Ok(input) => input,
            Err(err) => {
                tracing::error!(error = ?err, "failed to build proxy input");
                return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response());
            }
        };

        let resolver = match ProxyResolver::from_uri(&parts.uri, self.filter.clone()).await {
            Ok(resolver) => resolver,
            Err(err) => {
                tracing::error!(error = ?err, "failed to build proxy resolver");
                return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response());
            }
        };

        let should_apply = match resolver.evaluate(&input) {
            Ok(result) => result,
            Err(err) => {
                tracing::error!(error = ?err, "failed to evaluate proxy rules");
                return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response());
            }
        };
        let body_bytes = bytes.clone();
        let mut parts = parts;
        parts.extensions.insert(ProxyDecision {
            resolver: Arc::new(resolver),
            should_apply,
            bytes,
        });

        let request = Request::from_parts(parts, Body::from(body_bytes));
        match self.inner.serve(request).await {
            Ok(response) => Ok(response),
            Err(err) => match err {},
        }
    }
}
