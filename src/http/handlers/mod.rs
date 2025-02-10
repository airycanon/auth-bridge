use crate::http::{Context, HttpResult};
use anyhow::Result;
use futures::future::BoxFuture;
use http::{Request, Response};
use std::fmt::Debug;

pub mod log;
pub mod proxy;

pub trait HttpHandler<B>: Send + Sync + 'static
where
    B: Send + Debug + 'static,
{
    fn handle_request(
        &self,
        _context: &Context,
        request: Request<B>,
    ) -> BoxFuture<'static, Result<HttpResult<B>>> {
        Box::pin(async { Ok(HttpResult::Request(request)) })
    }

    fn handle_response(
        &self,
        _context: &Context,
        response: Response<B>,
    ) -> BoxFuture<'static, Result<Response<B>>> {
        Box::pin(async { Ok(response) })
    }
}
