use crate::proxy::body::ProxyBody;
use anyhow::Result;
use bytes::Bytes;
use http::request::Parts;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;

pub type InjectedResult<'a> = Pin<Box<dyn Future<Output = Result<Option<ProxyBody>>> + Send + 'a>>;

pub trait Injector: Send + Debug {
    fn inject<'a>(&'a self, parts: &'a mut Parts, bytes: &'a Bytes) -> InjectedResult<'a>;
}

pub mod auth;
pub use auth::AuthInjector;

pub mod body;
pub use body::BodyInjector;

pub mod header;
pub use header::HeaderInjector;

pub mod query;
pub use query::QueryInjector;
