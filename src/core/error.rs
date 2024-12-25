use http::{Response, StatusCode};
use hyper::body::Body;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
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

impl<B> From<Error> for Response<B>
where
    B: Body + From<String>,
{
    fn from(error: Error) -> Self {
        let status = match error {
            Error::MissingConnectInfo => StatusCode::NOT_FOUND,
            Error::UpstreamError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::BuildInput(_) => StatusCode::BAD_REQUEST,
            Error::ProxyResource(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::HandleProxy(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let mut response = Response::new(B::from(error.to_string()));
        *response.status_mut() = status;

        response
    }
}
