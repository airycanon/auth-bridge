use crate::core::layer::decision::ProxyDecision;
use rama::{
    Layer, Service,
    http::{Body, Request, Response, StatusCode},
    http::service::web::response::IntoResponse,
    telemetry::tracing,
};
use std::convert::Infallible;

#[derive(Clone, Debug, Default)]
pub struct InjectLayer;

impl<S> Layer<S> for InjectLayer {
    type Service = InjectService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        InjectService { inner }
    }
}

#[derive(Clone, Debug)]
pub struct InjectService<S> {
    inner: S,
}

impl<S> Service<Request> for InjectService<S>
where
    S: Service<Request, Output = Response, Error = Infallible>,
{
    type Output = Response;
    type Error = Infallible;

    async fn serve(&self, req: Request) -> std::result::Result<Self::Output, Self::Error> {
        let (parts, body) = req.into_parts();
        let decision = parts.extensions.get::<ProxyDecision>().cloned();
        let request = if let Some(decision) = decision {
            let mut http_parts: http::request::Parts = parts.into();
            let body = if decision.should_apply {
                match decision
                    .resolver
                    .as_ref()
                    .apply(&mut http_parts, &decision.bytes)
                    .await
                {
                    Ok(Some(proxy_body)) => match Body::try_from(proxy_body) {
                        Ok(body) => body,
                        Err(err) => {
                            tracing::error!(error = ?err, "failed to convert proxy body");
                            return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response());
                        }
                    },
                    Ok(None) => body,
                    Err(err) => {
                        tracing::error!(error = ?err, "failed to apply proxy");
                        return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response());
                    }
                }
            } else {
                body
            };
            let parts: rama::http::request::Parts = http_parts.into();
            Request::from_parts(parts, body)
        } else {
            Request::from_parts(parts, body)
        };

        match self.inner.serve(request).await {
            Ok(response) => Ok(response),
            Err(err) => match err {},
        }
    }
}
