use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;
use headers::{Authorization, HeaderMapExt, HeaderName, HeaderValue};
use hudsucker::hyper::Request;
use crate::apis::proxy_policy::{
    ProxyPolicy,
    ProxyPolicyMethod::{
        BasicAuth,
        BearerToken,
        CustomHeader,
        Query,
    },
};
use crate::secret::provider::provider;
use anyhow::{anyhow, Result};
use hudsucker::Body;
use hyper::Uri;

pub trait Injector {
    fn inject(&self, data: Cow<BTreeMap<String, String>>, request: &mut Request<Body>) -> Result<()>;
}

pub struct BasicAuthInjector {}

impl Injector for BasicAuthInjector {
    fn inject(&self, data: Cow<BTreeMap<String, String>>, request: &mut Request<Body>) -> Result<()> {
        let username = data.get("username").ok_or(anyhow!("username required"))?;
        let password = data.get("password").ok_or(anyhow!("password required"))?;

        Ok(request.headers_mut().typed_insert(Authorization::basic(username, password)))
    }
}

pub struct BearerTokenInjector {}

impl Injector for BearerTokenInjector {
    fn inject(&self, data: Cow<BTreeMap<String, String>>, request: &mut Request<Body>) -> Result<()> {
        let token = data.get("token").ok_or(anyhow!("token required"))?;
        let auth = Authorization::bearer(token)?;

        Ok(request.headers_mut().typed_insert(auth))
    }
}

pub struct CustomHeaderInjector {}

impl Injector for CustomHeaderInjector {
    fn inject(&self, data: Cow<BTreeMap<String, String>>, request: &mut Request<Body>) -> Result<()> {
        if let Some((key, value)) = data.first_key_value() {
            let header_key = HeaderName::try_from(key)?;
            let header_value = HeaderValue::try_from(value)?;
            request.headers_mut().insert(header_key, header_value);

            Ok(())
        } else {
            Err(anyhow!("secret is empty"))
        }
    }
}

pub struct QueryInjector {}

impl Injector for QueryInjector {
    fn inject(&self, data: Cow<BTreeMap<String, String>>, request: &mut Request<Body>) -> Result<()> {
        if let Some((key, value)) = data.first_key_value() {
            let uri = request.uri();
            let mut parts = uri.clone().into_parts();

            let query = parts.path_and_query
                .as_ref()
                .and_then(|pq| pq.query())
                .unwrap_or_default();

            let new_query = if query.is_empty() {
                format!("{}={}", key, value)
            } else {
                format!("{}&{}={}", query, key, value)
            };
            parts.path_and_query = Some(new_query.parse()?);

            let new_uri = Uri::from_parts(parts)?;
            *request.uri_mut() = new_uri;

            Ok(())
        } else {
            Err(anyhow!("secret is empty"))
        }
    }
}


pub async fn inject(request: &mut Request<Body>, policy: &ProxyPolicy) -> Result<()> {
    let provider = provider(&policy.spec.auth)?;
    let data = provider.secret().await?;

    let injector: Arc<dyn Injector> = match &policy.spec.auth.method {
        BasicAuth => Arc::new(BasicAuthInjector {}),
        BearerToken => Arc::new(BearerTokenInjector {}),
        CustomHeader => Arc::new(CustomHeaderInjector {}),
        Query => Arc::new(QueryInjector {})
    };

    injector.inject(data, request)
}