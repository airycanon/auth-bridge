use hudsucker::{Body, HttpContext, HttpHandler, RequestOrResponse};
use hudsucker::hyper::{Request, Response};
use log::info;

#[derive(Clone)]
pub struct LogHandler;

impl HttpHandler for LogHandler {
    async fn handle_request(
        &mut self,
        _ctx: &HttpContext,
        req: Request<Body>,
    ) -> RequestOrResponse {
        info!("{:?}", req);
        req.into()
    }

    async fn handle_response(&mut self, _ctx: &HttpContext, res: Response<Body>) -> Response<Body> {
        info!("{:?}", res);
        res
    }
}