use crate::http::handlers::{Context, HttpHandler, HttpResult, Result};
use futures::future::BoxFuture;
use hudsucker::hyper::{Request, Response};
use log::info;
use std::fmt::Debug;
use std::marker::PhantomData;

#[derive(Debug, Default)]
pub struct LogHandler<B>
where
    B: Send + Debug + 'static,
{
    phantom_data: PhantomData<B>,
}

impl<B> LogHandler<B>
where
    B: Send + Debug + 'static,
{
    pub fn new() -> Self {
        Self {
            phantom_data: PhantomData,
        }
    }
}

impl<B> HttpHandler<B> for LogHandler<B>
where
    B: Debug + Send + 'static,
{
    fn handle_request(
        &self,
        _ctx: &Context,
        request: Request<B>,
    ) -> BoxFuture<'static, Result<HttpResult<B>>> {
        info!("log request {:?}", request);
        Box::pin(async { Ok(HttpResult::Request(request)) })
    }

    fn handle_response(
        &self,
        _ctx: &Context,
        response: Response<B>,
    ) -> BoxFuture<'static, Result<Response<B>>> {
        info!("log response {:?}", response);
        Box::pin(async { Ok(response) })
    }
}

unsafe impl<B> Sync for LogHandler<B> where B: Send + Debug + 'static {}
