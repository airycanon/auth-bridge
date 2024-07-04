use hudsucker::{Body, HttpContext, HttpHandler, RequestOrResponse};
use hudsucker::hyper::Request;
use crate::handlers::log::LogHandler;
use crate::handlers::policy::{PolicyHandler};

#[derive(Clone,Default)]
pub struct MultiHandler {
    handlers: Vec<HandlerEnum>,
}

impl MultiHandler {
    pub fn new(handlers: Vec<HandlerEnum>) -> Self {
        MultiHandler {
            handlers
        }
    }
}

impl HttpHandler for MultiHandler {
    async fn handle_request(&mut self, ctx: &HttpContext, mut req: Request<Body>) -> RequestOrResponse {
        for handler in &self.handlers {
            match handler.handle_request(ctx, req).await {
                RequestOrResponse::Request(new_req) => req = new_req,
                response => return response,
            }
        }
        RequestOrResponse::Request(req)
    }
}
#[derive(Clone)]
pub enum HandlerEnum {
    Log,
    Policy
}

impl HandlerEnum {
    async fn handle_request(&self, ctx: &HttpContext, req: Request<Body>) -> RequestOrResponse {
        let res = match self {
            HandlerEnum::Log => LogHandler.handle_request(ctx, req).await,
            HandlerEnum::Policy => PolicyHandler.handle_request(ctx, req).await,
        };

        res
    }
}