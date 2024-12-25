use std::net::SocketAddr;
use futures::future::BoxFuture;
use http::{Request, Response};
use anyhow::Result;

pub mod chain;
pub mod auth;

pub mod body;

pub mod proxy;
pub mod log;

#[derive(Debug)]
pub enum HttpResult<B> {
    /// HTTP Request
    Request(Request<B>),
    /// HTTP Response
    Response(Response<B>),
}
type RequestHandler<B> = fn(&Context, Request<B>) -> BoxFuture<'static, Result<HttpResult<B>>>;
type ResponseHandler<B> = fn(&Context, Response<B>) -> BoxFuture<'static, Result<Response<B>>>;

pub struct Context {
    pub addr: SocketAddr,
}

impl Context {
    pub fn new(addr: SocketAddr) -> Context {
        Self { addr }
    }
}
