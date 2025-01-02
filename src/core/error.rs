use http::{Response, StatusCode};
use hyper::body::Body;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("missing connect info")]
    MissingConnectInfo,

    #[error(transparent)]
    HandleProxy(#[from] anyhow::Error),

    #[error(transparent)]
    UpstreamError(#[from] hyper_util::client::legacy::Error),

    #[error(transparent)]
    BuildInput(#[from] serde_json::Error),
    #[error(transparent)]
    ProxyResource(#[from] kube::error::Error),
}

impl<B> From<ProxyError> for Response<B>
where
    B: Body + From<String>,
{
    fn from(error: ProxyError) -> Self {
        let status = match error {
            ProxyError::MissingConnectInfo => StatusCode::NOT_FOUND,
            ProxyError::UpstreamError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ProxyError::BuildInput(_) => StatusCode::BAD_REQUEST,
            ProxyError::ProxyResource(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ProxyError::HandleProxy(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let mut response = Response::new(B::from(error.to_string()));
        *response.status_mut() = status;

        response
    }
}
