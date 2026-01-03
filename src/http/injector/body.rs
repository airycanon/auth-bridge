use crate::http::body::ProxyBody;
use crate::http::injector::{InjectedResult, Injector};
use bytes::Bytes;
use http::request::Parts;
use std::fmt;

pub struct BodyInjector {
    key: String,
    value: String,
}

impl BodyInjector {
    pub fn new(key: String, value: String) -> BodyInjector {
        Self { key, value }
    }
}

impl Injector for BodyInjector {
    fn inject<'a>(&'a self, parts: &'a mut Parts, bytes: &'a Bytes) -> InjectedResult<'a> {
        Box::pin(async move {
            let content_type = parts
                .headers
                .get(http::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default()
                .to_string();
            let mut body = ProxyBody::from_bytes(content_type, bytes.clone());
            body.insert(self.key.to_string(), self.value.to_string());

            Ok(Some(body))
        })
    }
}

impl fmt::Debug for BodyInjector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BodyInjector")
            .field("key", &self.key)
            .field("value", &"*****")
            .finish()
    }
}
