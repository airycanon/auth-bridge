use crate::proxy::injector::{InjectedResult, Injector};
use bytes::Bytes;
use headers::{Authorization, HeaderMapExt};
use http::request::Parts;
use std::fmt;

pub struct BearerTokenInjector {
    token: String,
}

impl BearerTokenInjector {
    pub fn new(token: String) -> BearerTokenInjector {
        Self { token }
    }
}

impl Injector for BearerTokenInjector {
    fn inject<'a>(&'a self, parts: &'a mut Parts, _bytes: &'a Bytes) -> InjectedResult<'a> {
        Box::pin(async move {
            let auth = Authorization::bearer(self.token.as_str())?;
            parts.headers.typed_insert(auth);

            Ok(None)
        })
    }
}

impl fmt::Debug for BearerTokenInjector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BearerTokenInjector")
            .field("token", &"*****")
            .finish()
    }
}
