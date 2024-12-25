use crate::http::body::ProxyBody;
use crate::http::{Context, HttpResult, Result};
use futures::future::BoxFuture;
use hudsucker::hyper::{Request, Response};
use log::info;
use std::fmt::Debug;

pub fn log_request<B>(_ctx: &Context, req: Request<B>) -> BoxFuture<'static, Result<HttpResult<B>>>
where
    B: Send + Debug + TryFrom<ProxyBody> + 'static,
{
    info!("{:?}", req);
    Box::pin(async { Ok(HttpResult::Request(req)) })
}

pub fn log_response<B>(_ctx: &Context, res: Response<B>) -> BoxFuture<'static, Result<Response<B>>>
where
    B: Send + Debug + TryFrom<ProxyBody> + 'static,
{
    info!("{:?}", res);
    Box::pin(async { Ok(res) })
}
