use http::{Response, StatusCode};
use hudsucker::{Body, RequestOrResponse};

pub struct ApiError<E>(pub E);

impl<E: std::fmt::Display> From<ApiError<E>> for RequestOrResponse {
    fn from(err: ApiError<E>) -> Self {
        from_error(err.0, StatusCode::INTERNAL_SERVER_ERROR)
    }
}

pub struct ParseError<E>(pub E);

impl<E: std::fmt::Display> From<ParseError<E>> for RequestOrResponse {
    fn from(err: ParseError<E>) -> Self {
        from_error(err.0, StatusCode::BAD_REQUEST)
    }
}

fn from_error<E: std::fmt::Display>(err: E, status_code: StatusCode) -> RequestOrResponse {
    let error_message = err.to_string();
    let mut res = Response::new(Body::from(error_message));
    *res.status_mut() = status_code;
    RequestOrResponse::Response(res)
}
