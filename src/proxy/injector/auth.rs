use crate::proxy::injector::{InjectedResult, Injector};
use anyhow::Result;
use bytes::Bytes;
use http::request::Parts;
use rama::http::headers::authorization::Credentials;
use rama::http::headers::{Authorization, HeaderMapExt};
use rama::net::user::credentials::{Basic, Bearer};
use rama::utils::str::NonEmptyStr;
use std::fmt;

/// Injects an `Authorization` header for any credential scheme `C`.
///
/// The credential types (`Basic`, `Bearer`, …) redact their secrets in their
/// own `Debug` impls, so the derived `Debug` here is safe to log.
#[derive(Debug)]
pub struct AuthInjector<C> {
    authorization: Authorization<C>,
}

impl<C> AuthInjector<C> {
    pub fn new(authorization: Authorization<C>) -> Self {
        Self { authorization }
    }
}

impl AuthInjector<Basic> {
    pub fn basic(username: &str, password: &str) -> Result<Self> {
        let username = NonEmptyStr::try_from(username)?;
        let password = NonEmptyStr::try_from(password)?;
        Ok(Self::new(Authorization::new(Basic::new(username, password))))
    }
}

impl AuthInjector<Bearer> {
    pub fn bearer(token: &str) -> Result<Self> {
        let token = NonEmptyStr::try_from(token)?;
        let bearer = Bearer::try_new(token).map_err(anyhow::Error::msg)?;
        Ok(Self::new(Authorization::new(bearer)))
    }
}

impl<C> Injector for AuthInjector<C>
where
    C: Credentials + Clone + fmt::Debug + Send + Sync + 'static,
{
    fn inject<'a>(&'a self, parts: &'a mut Parts, _bytes: &'a Bytes) -> InjectedResult<'a> {
        Box::pin(async move {
            parts.headers.typed_insert(self.authorization.clone());

            Ok(None)
        })
    }
}