use crate::proxy::injector::{InjectedResult, Injector};
use bytes::Bytes;
use headers::{Authorization, HeaderMapExt};
use http::request::Parts;
use std::fmt;

pub struct BasicAuthInjector {
    username: String,
    password: String,
}

impl BasicAuthInjector {
    pub fn new(username: String, password: String) -> BasicAuthInjector {
        Self { username, password }
    }
}

impl Injector for BasicAuthInjector {
    fn inject<'a>(&'a self, parts: &'a mut Parts, _bytes: &'a Bytes) -> InjectedResult<'a> {
        Box::pin(async move {
            parts.headers.typed_insert(Authorization::basic(
                self.username.as_str(),
                self.password.as_str(),
            ));

            Ok(None)
        })
    }
}

impl fmt::Debug for BasicAuthInjector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BasicAuthInjector")
            .field("username", &self.username)
            .field("password", &"*****")
            .finish()
    }
}
