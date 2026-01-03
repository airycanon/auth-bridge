use rama::{
    Layer, Service,
    http::{Request, Response, StatusCode, Uri, header, uri::Scheme},
    http::service::web::response::IntoResponse,
    telemetry::tracing,
};
use std::convert::Infallible;

#[derive(Clone, Debug, Default)]
pub struct NormalizeLayer;

impl<S> Layer<S> for NormalizeLayer {
    type Service = NormalizeService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        NormalizeService { inner }
    }
}

#[derive(Clone, Debug)]
pub struct NormalizeService<S> {
    inner: S,
}

impl<S> Service<Request> for NormalizeService<S>
where
    S: Service<Request, Output = Response, Error = Infallible>,
{
    type Output = Response;
    type Error = Infallible;

    async fn serve(&self, req: Request) -> std::result::Result<Self::Output, Self::Error> {
        let (mut parts, body) = req.into_parts();

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

        match builder.build() {
            Ok(uri) => parts.uri = uri,
            Err(err) => {
                tracing::error!(error = ?err, "failed to normalize uri");
                return Ok(StatusCode::BAD_REQUEST.into_response());
            }
        }

        tracing::info!(uri = %parts.uri, "proxy request url");

        let request = Request::from_parts(parts, body);
        match self.inner.serve(request).await {
            Ok(response) => Ok(response),
            Err(err) => match err {},
        }
    }
}
