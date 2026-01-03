use crate::http::injector::{InjectedResult, Injector};
use bytes::Bytes;
use http::request::Parts;
use http::Uri;
use std::fmt;

pub struct QueryInjector {
    key: String,
    value: String,
}

impl QueryInjector {
    pub fn new(key: String, value: String) -> QueryInjector {
        Self { key, value }
    }
}

impl Injector for QueryInjector {
    fn inject<'a>(&'a self, parts: &'a mut Parts, _bytes: &'a Bytes) -> InjectedResult<'a> {
        Box::pin(async move {
            let uri = parts.uri.clone();
            let mut uri_parts = uri.clone().into_parts();

            let query = uri_parts
                .path_and_query
                .as_ref()
                .and_then(|pq| pq.query())
                .unwrap_or_default();

            let new_query = if query.is_empty() {
                format!("{}={}", self.key, self.value)
            } else {
                format!("{}&{}={}", query, self.key, self.value)
            };
            uri_parts.path_and_query = Some(new_query.parse()?);

            let new_uri = Uri::from_parts(uri_parts)?;
            parts.uri = new_uri;

            Ok(None)
        })
    }
}

impl fmt::Debug for QueryInjector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QueryInjector")
            .field("key", &self.key)
            .field("value", &"*****")
            .finish()
    }
}
