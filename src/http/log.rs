use crate::http::body::ProxyBody;
use crate::http::{Context, HttpResult, Result};
use futures::future::BoxFuture;
use hudsucker::hyper::{Request, Response};
use log::info;
use std::fmt::Debug;

pub fn log_request<'a, B>(_ctx: &Context, req: Request<B>) -> BoxFuture<'a, Result<HttpResult<B>>>
where
    B: Send + Debug + TryFrom<ProxyBody> + 'a,
{
    info!("log request {:?}", req);
    Box::pin(async { Ok(HttpResult::Request(req)) })
}

pub fn log_response<'a, B>(_ctx: &Context, res: Response<B>) -> BoxFuture<'a, Result<Response<B>>>
where
    B: Send + Debug + TryFrom<ProxyBody> + 'a,
{
    info!("log response {:?}", res);
    Box::pin(async { Ok(res) })
}
