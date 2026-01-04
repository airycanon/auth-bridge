use crate::runtime::pod::store::Store;
use anyhow::Result;
use bytes::Bytes;
use http::header::CONTENT_TYPE;
use rama::http::request::Parts;
use serde_json::Value;
use std::collections::BTreeMap;
use std::net::IpAddr;


#[derive(Clone, Debug)]
pub struct Input(pub BTreeMap<String, Value>);

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
pub struct InputBuilder<'a> {
    query: Option<BTreeMap<String, String>>,
    headers: Option<BTreeMap<String, String>>,
    uri: Option<String>,
    body: Option<&'a Bytes>,
    ip: Option<IpAddr>,
    content_type: String,
}

impl<'a> InputBuilder<'a> {
    pub fn with_parts_ref(mut self, parts: &Parts) -> Self {
        let content_type = parts
            .headers
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        self.content_type = content_type.to_string();

        let query: BTreeMap<String, String> = parts
            .uri
            .query()
            .map(|v| form_urlencoded::parse(v.as_bytes()).into_owned().collect())
            .unwrap_or_else(BTreeMap::new);
        self.query = Some(query);

        let headers: BTreeMap<String, String> = parts
            .headers
            .iter()
            .filter_map(|(k, v)| {
                let value = v.to_str().ok()?;
                Some((k.to_string(), value.to_string()))
            })
            .collect();
        self.headers = Some(headers);

        self.uri = Some(parts.uri.to_string());
        self
    }

    pub fn with_body_ref(mut self, bytes: &'a Bytes) -> Self {
        self.body = Some(bytes);
        self
    }

    pub fn with_pod_ip(mut self, ip: IpAddr) -> Self {
        self.ip = Some(ip);
        self
    }

    pub fn build(self) -> Result<Input> {
        let mut input = BTreeMap::new();

        if let Some(query) = self.query {
            input.insert(String::from("query"), serde_json::to_value(query)?);
        }

        if let Some(headers) = self.headers {
            input.insert(String::from("headers"), serde_json::to_value(headers)?);
        }

        if let Some(uri) = self.uri {
            input.insert(String::from("uri"), Value::from(uri));
        }

        if let Some(bytes) = self.body {
            let map = match &self.content_type {
                t if t.starts_with("application/x-www-form-urlencoded") => form_urlencoded::parse(bytes)
                    .into_owned()
                    .map(|(k, v)| (k.clone(), Value::String(v)))
                    .collect::<BTreeMap<String, Value>>(),
                t if t.starts_with("application/json") => {
                    let value: Value = serde_json::from_slice(bytes)?;
                    value
                        .as_object()
                        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                        .unwrap_or_default()
                }
                _ => BTreeMap::new(),
            };
            input.insert("body".to_string(), serde_json::to_value(map)?);
        }

        if let Some(ip) = self.ip {
            if let Some(meta) = Store::global().find(&ip.to_string()) {
                input.insert(String::from("meta"), serde_json::to_value(meta.as_ref())?);
            }
        }

        Ok(Input(input))
    }
}
