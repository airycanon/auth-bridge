use rama::{
    Layer, Service,
    http::{Request, Response},
    telemetry::tracing,
};
use std::convert::Infallible;

#[derive(Clone, Debug, Default)]
pub struct LogLayer;

impl<S> Layer<S> for LogLayer {
    type Service = LogService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        LogService { inner }
    }
}

#[derive(Clone, Debug)]
pub struct LogService<S> {
    inner: S,
}

impl<S> Service<Request> for LogService<S>
where
    S: Service<Request, Output = Response, Error = Infallible>,
{
    type Output = Response;
    type Error = Infallible;

    async fn serve(&self, req: Request) -> Result<Self::Output, Self::Error> {
        tracing::info!(method = %req.method(), uri = %req.uri(), "mitm request");

        let response = match self.inner.serve(req).await {
            Ok(response) => response,
            Err(err) => match err {},
        };

        tracing::info!(status = %response.status(), "mitm response");

        Ok(response)
    }
}
