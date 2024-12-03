use crate::proxy::forward::multi::{HttpHandler, ProxyResponse, ProxyResult};
use hudsucker::hyper::{Request, Response};
use hudsucker::{Body, HttpContext};
use log::info;

#[derive(Clone, Default)]
pub struct LogHandler;

impl HttpHandler for LogHandler {
    fn handle_request(&self, _ctx: &HttpContext, req: Request<Body>) -> ProxyResult {
        info!("{:?}", req);
        Box::pin(async { req.into() })
    }

    fn handle_response(&self, _ctx: &HttpContext, res: Response<Body>) -> ProxyResponse {
        info!("{:?}", res);
        Box::pin(async { res })
    }
}
