use crate::http::injector::{InjectedResult, Injector};
use bytes::Bytes;
use http::{HeaderName, HeaderValue};
use http::request::Parts;
use std::fmt;
use std::str::FromStr;

pub struct HeaderInjector {
    key: String,
    value: String,
}

impl HeaderInjector {
    pub fn new(key: String, value: String) -> HeaderInjector {
        Self { key, value }
    }
}

impl Injector for HeaderInjector {
    fn inject<'a>(&'a self, parts: &'a mut Parts, _bytes: &'a Bytes) -> InjectedResult<'a> {
        let key = self.key.clone();
        let value = self.value.clone();

        Box::pin(async move {
            let header_name = HeaderName::from_str(&key)?;
            let header_value = HeaderValue::from_str(&value)?;

            parts.headers.insert(header_name, header_value);

            Ok(None)
        })
    }
}

impl fmt::Debug for HeaderInjector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HeaderInjector")
            .field("key", &self.key)
            .field("value", &"*****")
            .finish()
    }
}
