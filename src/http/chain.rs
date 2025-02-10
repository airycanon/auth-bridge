use crate::core::error::ProxyError;
use crate::core::error::ProxyError::MissingConnectInfo;
use crate::http::body::ProxyBody;
use crate::http::handlers::HttpHandler;
use crate::http::Context;
use crate::http::{HttpResult, Result};
use axum::body::Body as ReverseBody;
use axum::extract::{ConnectInfo, State};
use axum::response::IntoResponse;
use futures::future::BoxFuture;
use http::{Request, Response};
use hudsucker::{Body as ForwardBody, HttpContext, RequestOrResponse};
use hyper_util::client::legacy::connect::HttpConnector;
use log::debug;
use std::fmt::Debug;
use std::future::Future;
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::sync::Arc;

type Client = hyper_util::client::legacy::Client<HttpConnector, ReverseBody>;
pub struct Chain<B, H>
where
    B: Send + Debug + 'static,
    H: HttpHandler<B> + ?Sized + Send + Sync + 'static,
{
    handlers: Vec<Arc<H>>,
    phantom: PhantomData<B>,
}

impl<B, H> Chain<B, H>
where
    B: Send + Debug + TryFrom<ProxyBody> + 'static,
    H: HttpHandler<B> + ?Sized + Send + Sync + 'static,
{
    pub fn new(handlers: Vec<Arc<H>>) -> Self {
        Self {
            handlers,
            phantom: Default::default(),
        }
    }

    fn process_request<'a>(
        &'a self,
        ctx: &'a Context,
        request: Request<B>,
    ) -> impl Future<Output = Result<HttpResult<B>>> + Send + 'a {
        let handlers = self.handlers.clone();
        async move {
            let mut current = request;
            for handler in handlers {
                match handler.handle_request(ctx, current).await? {
                    HttpResult::Request(req) => current = req,
                    response => return Ok(response),
                }
            }
            debug!("handler process request done: {:?}", current);
            Ok(HttpResult::Request(current))
        }
    }

    fn process_response<'a>(
        &'a self,
        ctx: &'a Context,
        response: Response<B>,
    ) -> impl Future<Output = Result<Response<B>>> + Send + 'a {
        let handlers = self.handlers.clone();

        async move {
            let mut current = response;
            for handler in handlers {
                current = handler.handle_response(ctx, current).await?;
            }
            debug!("handler process response done: {:?}", current);
            Ok(current)
        }
    }
}

impl<B, H> Default for Chain<B, H>
where
    B: Send + Debug + TryFrom<ProxyBody> + 'static,
    H: HttpHandler<B> + ?Sized + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl<B, H> Clone for Chain<B, H>
where
    B: Send + Debug + TryFrom<ProxyBody> + 'static,
    H: HttpHandler<B> + ?Sized + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self::new(self.handlers.clone())
    }
}

unsafe impl<B, H> Sync for Chain<B, H>
where
    B: Send + Debug + TryFrom<ProxyBody> + 'static,
    H: HttpHandler<B> + ?Sized + Send + Sync + 'static,
{
}

pub type ReverseChain = Chain<ReverseBody, dyn HttpHandler<ReverseBody>>;
pub type ForwardChain = Chain<ForwardBody, dyn HttpHandler<ForwardBody>>;

impl hudsucker::HttpHandler for ForwardChain {
    async fn handle_request(
        &mut self,
        ctx: &HttpContext,
        request: Request<ForwardBody>,
    ) -> RequestOrResponse {
        let context = Context::new(ctx.client_addr);
        match self.process_request(&context, request).await {
            Ok(HttpResult::Request(request)) => RequestOrResponse::Request(request),
            Ok(HttpResult::Response(response)) => RequestOrResponse::Response(response),
            Err(err) => RequestOrResponse::Response(ProxyError::from(err).into()),
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
            .unwrap_or_else(|err| ProxyError::from(err).into())
    }
}

impl<T> axum::handler::Handler<T, State<Client>> for ReverseChain {
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
                    Err(err) => return ProxyError::from(err).into(),
                },
                Ok(HttpResult::Response(response)) => response.into_response(),
                Err(err) => ProxyError::from(err).into(),
            };

            match self.process_response(&context, response).await {
                Ok(response) => response.into_response(),
                Err(err) => ProxyError::from(err).into(),
            }
        })
    }
}
