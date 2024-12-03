use http::{Request, Response};
use hudsucker::{Body, HttpContext, RequestOrResponse};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub type ProxyResult<'a> = Pin<Box<dyn Future<Output = RequestOrResponse> + Send + 'a>>;
pub type ProxyResponse<'a> = Pin<Box<dyn Future<Output = Response<Body>> + Send + 'a>>;

pub trait HttpHandler: Send + Sync {
    fn handle_request<'a>(&'a self, _: &'a HttpContext, request: Request<Body>) -> ProxyResult {
        Box::pin(async { RequestOrResponse::Request(request) })
    }

    fn handle_response<'a>(&'a self, _: &'a HttpContext, res: Response<Body>) -> ProxyResponse {
        Box::pin(async { res })
    }
}

pub struct MultiHandler {
    handlers: Vec<Arc<dyn HttpHandler>>,
}

impl MultiHandler {
    pub fn new(handlers: Vec<Arc<dyn HttpHandler>>) -> Self {
        Self { handlers }
    }
}

impl Clone for MultiHandler {
    fn clone(&self) -> Self {
        Self {
            handlers: self.handlers.clone(),
        }
    }
}

impl hudsucker::HttpHandler for MultiHandler {
    async fn handle_request(&mut self, ctx: &HttpContext, req: Request<Body>) -> RequestOrResponse {
        let mut current = req;
        for handler in &self.handlers {
            match handler.handle_request(ctx, current).await {
                RequestOrResponse::Request(new_req) => current = new_req,
                response @ RequestOrResponse::Response(_) => return response,
            }
        }
        RequestOrResponse::Request(current)
    }

    async fn handle_response(&mut self, ctx: &HttpContext, res: Response<Body>) -> Response<Body> {
        let mut current = res;
        for handler in &self.handlers {
            current = handler.handle_response(ctx, current).await;
        }
        current
    }
}
