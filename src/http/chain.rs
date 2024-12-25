use crate::http::{Context, RequestHandler, ResponseHandler};
use crate::http::{HttpResult, Result};
use axum::body::Body as ReverseBody;
use axum::extract::{ConnectInfo, State};
use axum::response::IntoResponse;
use futures::future::BoxFuture;
use http::{Request, Response};
use hudsucker::{Body as ForwardBody, HttpContext, RequestOrResponse};
use hyper_util::client::legacy::connect::HttpConnector;
use std::fmt::Debug;
use std::net::SocketAddr;
use crate::core::error::Error;
use crate::core::error::Error::MissingConnectInfo;
use crate::http::body::ProxyBody;

type Client = hyper_util::client::legacy::Client<HttpConnector, ReverseBody>;

pub struct Chain<B> {
    request_handlers: Vec<RequestHandler<B>>,
    response_handlers: Vec<ResponseHandler<B>>,
}

impl<B> Chain<B>
where
    B: Send + Debug + TryFrom<ProxyBody> + 'static,
{
    pub fn new() -> Self {
        Self {
            request_handlers: Vec::new(),
            response_handlers: Vec::new(),
        }
    }

    pub fn with_request_handler(mut self, handler: RequestHandler<B>) -> Self {
        self.request_handlers.push(handler);
        self
    }

    pub fn with_response_handler(mut self, handler: ResponseHandler<B>) -> Self {
        self.response_handlers.push(handler);
        self
    }

    async fn process_request(
        &self,
        context: &Context,
        request: Request<B>,
    ) -> Result<HttpResult<B>> {
        let mut current = request;
        for handler in &self.request_handlers {
            match handler(context, current).await? {
                HttpResult::Request(req) => current = req,
                response => return Ok(response),
            }
        }
        Ok(HttpResult::Request(current))
    }

    async fn process_response(
        &self,
        context: &Context,
        response: Response<B>,
    ) -> Result<Response<B>> {
        let mut current = response;
        for handler in &self.response_handlers {
            current = handler(context, current).await?;
        }
        Ok(current)
    }
}

impl<B> Default for Chain<B>
where
    B: Send + Debug + TryFrom<ProxyBody> + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

// 实现 Clone
impl<B> Clone for Chain<B> {
    fn clone(&self) -> Self {
        Self {
            request_handlers: self.request_handlers.clone(),
            response_handlers: self.response_handlers.clone(),
        }
    }
}

// 为 hudsucker 实现
impl hudsucker::HttpHandler for Chain<ForwardBody> {
    async fn handle_request(
        &mut self,
        ctx: &HttpContext,
        request: Request<ForwardBody>,
    ) -> RequestOrResponse {
        let context = Context::new(ctx.client_addr);
        match self.process_request(&context, request).await {
            Ok(HttpResult::Request(request)) => RequestOrResponse::Request(request),
            Ok(HttpResult::Response(response)) => RequestOrResponse::Response(response),
            Err(err) => RequestOrResponse::Response(Error::from(err).into()),
        }
    }

    async fn handle_response(
        &mut self,
        ctx: &HttpContext,
        response: Response<ForwardBody>,
    ) -> Response<ForwardBody> {
        let context = Context::new(ctx.client_addr);
        self.process_response(&context, response)
            .await
            .unwrap_or_else(|err| Error::from(err).into())
    }
}

// 为 axum 实现
impl<T> axum::handler::Handler<T, State<Client>> for Chain<ReverseBody> {
    type Future = BoxFuture<'static, Response<ReverseBody>>;

    fn call(self, request: Request<ReverseBody>, State(client): State<Client>) -> Self::Future {
        Box::pin(async move {
            let connect_info = match request.extensions().get::<ConnectInfo<SocketAddr>>() {
                Some(info) => info,
                None => return Response::<ReverseBody>::from(MissingConnectInfo),
            };

            let context = Context::new(connect_info.0);
            let response = match self.process_request(&context, request).await {
                Ok(HttpResult::Request(request)) => match client.request(request).await {
                    Ok(response) => response.into_response(),
                    Err(err) => return Error::from(err).into(),
                },
                Ok(HttpResult::Response(response)) => response.into_response(),
                Err(err) => Error::from(err).into(),
            };

            match self.process_response(&context, response).await {
                Ok(response) => response.into_response(),
                Err(err) => Error::from(err).into(),
            }
        })
    }
}
