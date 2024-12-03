use crate::core::body::ProxyBody;
use anyhow::Result;
use bytes::Bytes;
use http::header::CONTENT_TYPE;
use http::request::Parts;
use http::Uri;
use serde_json::Value;
use std::collections::BTreeMap;
use std::net::IpAddr;
use crate::core::pod::store::Store;

#[derive(Clone)]
pub struct Input(BTreeMap<String, Value>);

impl FromIterator<(String, Value)> for Input {
    fn from_iter<I: IntoIterator<Item = (String, Value)>>(iter: I) -> Self {
        Input(BTreeMap::from_iter(iter))
    }
}

impl From<Input> for regorus::Value {
    fn from(input: Input) -> Self {
        let value = serde_json::to_value(input.0).unwrap();
        regorus::Value::from(value)
    }
}

impl<T> From<BTreeMap<String, T>> for Input
where
    Value: for<'a> From<&'a T>,
{
    fn from(data: BTreeMap<String, T>) -> Self {
        data.iter()
            .map(|(key, value)| (key.clone(), Value::from(value)))
            .collect::<Input>()
    }
}

#[derive(Debug, Default)]
pub struct InputBuilder {
    uri: Option<Uri>,
    body: Option<Bytes>,
    ip: Option<IpAddr>,
    content_type: String,
}

impl InputBuilder {
    pub fn with_uri(mut self, uri: Uri) -> Self {
        self.uri = Some(uri);
        self
    }

    pub fn with_body(mut self, parts: Parts, bytes: Bytes) -> Self {
        let content_type = parts
            .headers
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        self.content_type = content_type.to_string();
        self.body = Some(bytes);
        self
    }

    pub fn with_pod_ip(mut self, ip: IpAddr) -> Self {
        self.ip = Some(ip);
        self
    }

    pub fn build(self) -> Result<Input> {
        let mut input = BTreeMap::new();

        if let Some(uri) = self.uri.clone() {
            let query: BTreeMap<String, String> = uri
                .query()
                .map(|v| form_urlencoded::parse(v.as_bytes()).into_owned().collect())
                .unwrap_or_else(BTreeMap::new);

            input.insert("query".to_string(), serde_json::to_value(query)?);
        }

        if let Some(bytes) = self.body {
            let body = ProxyBody::from_bytes(self.content_type, bytes);
            input.insert("body".to_string(), body.try_into()?);
        }

        if let Some(uri) = self.uri {
            input.insert("uri".to_string(), Value::from(uri.to_string()));
        }

        if let Some(ip) = self.ip {
            if let Some(meta) = Store::global().find(&ip.to_string()) {
                input.insert("meta".to_string(), serde_json::to_value(meta.as_ref())?);
            }
        }

        Ok(Input(input))
    }
}
