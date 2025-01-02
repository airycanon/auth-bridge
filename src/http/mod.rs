use anyhow::Result;
use futures::future::BoxFuture;
use http::{Request, Response};
use std::net::SocketAddr;

pub mod auth;
pub mod chain;

pub mod body;

pub mod log;
pub mod proxy;

#[derive(Debug)]
pub enum HttpResult<B> {
    /// HTTP Request
    Request(Request<B>),
    /// HTTP Response
    Response(Response<B>),
}
type RequestHandler<'a, B> = fn(&Context, Request<B>) -> BoxFuture<'a, Result<HttpResult<B>>>;
type ResponseHandler<'a, B> = fn(&Context, Response<B>) -> BoxFuture<'a, Result<Response<B>>>;

pub struct Context {
    pub addr: SocketAddr,
}

impl Context {
    pub fn new(addr: SocketAddr) -> Context {
        Self { addr }
    }
}
