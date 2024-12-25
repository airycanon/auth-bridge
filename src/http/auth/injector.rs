use crate::http::body::ProxyBody;
use anyhow::Result;
use bytes::Bytes;
use headers::{Authorization, HeaderMapExt};
use http::request::Parts;
use http::{HeaderName, HeaderValue, Uri};
use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;

type InjectedRequest<'a> = Pin<Box<dyn Future<Output = Result<ProxyBody>> + Send + 'a>>;

pub trait Injector: Send {
    fn inject<'a>(&'a self, parts: &'a mut Parts, bytes: Bytes) -> InjectedRequest<'a>;
}

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
    fn inject<'a>(&'a self, parts: &'a mut Parts, bytes: Bytes) -> InjectedRequest<'a> {
        Box::pin(async move {
            parts.headers.typed_insert(Authorization::basic(
                self.username.as_str(),
                self.password.as_str(),
            ));

            Ok(ProxyBody::from(bytes))
        })
    }
}

pub struct BearerTokenInjector {
    token: String,
}

impl BearerTokenInjector {
    pub fn new(token: String) -> BearerTokenInjector {
        Self { token }
    }
}

impl Injector for BearerTokenInjector {
    fn inject<'a>(&'a self, parts: &'a mut Parts, bytes: Bytes) -> InjectedRequest<'a> {
        Box::pin(async move {
            let auth = Authorization::bearer(self.token.as_str())?;
            parts.headers.typed_insert(auth);

            Ok(ProxyBody::from(bytes))
        })
    }
}

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
    fn inject<'a>(&'a self, parts: &'a mut Parts, bytes: Bytes) -> InjectedRequest<'a> {
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

            Ok(ProxyBody::from(bytes))
        })
    }
}

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
    fn inject<'a>(&'a self, parts: &'a mut Parts, bytes: Bytes) -> InjectedRequest<'a> {
        let key = self.key.clone();
        let value = self.value.clone();

        Box::pin(async move {
            let header_name = HeaderName::from_str(&key)?;
            let header_value = HeaderValue::from_str(&value)?;

            parts.headers.insert(header_name, header_value);

            Ok(ProxyBody::from(bytes))
        })
    }
}

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
    fn inject<'a>(&'a self, parts: &'a mut Parts, bytes: Bytes) -> InjectedRequest<'a> {
        Box::pin(async move {
            let content_type = parts
                .headers
                .get(hyper::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default()
                .to_string();
            let mut body = ProxyBody::from_bytes(content_type, bytes);
            body.insert(self.key.to_string(), self.value.to_string());

            Ok(body)
        })
    }
}
