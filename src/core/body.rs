use bytes::Bytes;
use http_body_util::Full;
use serde_json::{Error, Value};
use std::collections::BTreeMap;
use std::result::Result;

pub const CONTENT_TYPE_FORM: &str = "application/x-www-form-urlencoded";

pub const CONTENT_TYPE_JSON: &str = "application/json";

pub struct ProxyBody {
    content_type: Option<String>,
    bytes: Bytes,
    values: BTreeMap<String, Value>,
}

impl ProxyBody {
    pub fn from_bytes(content_type: String, bytes: Bytes) -> Self {
        ProxyBody {
            content_type: Some(content_type),
            bytes,
            values: Default::default(),
        }
    }

    pub fn insert(&mut self, key: String, value: String) -> &mut ProxyBody {
        self.values
            .insert(key, Value::String(value));
        self
    }

    pub fn to_map(&self) -> Result<BTreeMap<String, Value>, Error> {
        let map = match &self.content_type {
            Some(t) if t.starts_with(CONTENT_TYPE_FORM) => form_urlencoded::parse(&self.bytes)
                .into_owned()
                .map(|(k, v)| (k.clone(), Value::String(v)))
                .collect::<BTreeMap<String, Value>>(),
            Some(t) if t.starts_with(CONTENT_TYPE_JSON) => {
                let value: Value = serde_json::from_slice(&self.bytes)?;
                value
                    .as_object()
                    .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                    .unwrap_or_default()
            }
            _ => BTreeMap::new(),
        };
        Ok(map)
    }

    pub fn bytes(self) -> Result<Bytes, Error> {
        if self.values.is_empty() {
            return Ok(self.bytes);
        }
        let mut data = self.to_map()?;
        data.extend(self.values);

        match self.content_type {
            Some(t) if t.starts_with(CONTENT_TYPE_FORM) => {
                let form_string = data
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<String>>()
                    .join("&");
                Ok(Bytes::from(form_string))
            }

            Some(t) if t.starts_with(CONTENT_TYPE_JSON) => {
                Ok(Bytes::from(serde_json::to_vec(&data)?))
            }
            _ => Ok(self.bytes),
        }
    }
}
//
// impl Body for ProxyBody {
//     type Data = Bytes;
//     type Error = ();
//
//     fn poll_frame(
//         mut self: Pin<&mut Self>,
//         cx: &mut std::task::Context<'_>,
//     ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
//         let bytes = self.get_mut().bytes()?;
//         let body = Full::from(self);
//         Pin::new(body.poll_frame(cx).map_err(|e| match e {}))
//     }
//
//     fn is_end_stream(&self) -> bool {
//         let body = Full::from(self);
//         body.is_end_stream()
//     }
//
//     fn size_hint(&self) -> SizeHint {
//         let body = Full::from(self);
//         body.size_hint()
//     }
// }

impl From<Bytes> for ProxyBody {
    fn from(bytes: Bytes) -> Self {
        ProxyBody {
            content_type: None,
            bytes,
            values: Default::default(),
        }
    }
}

impl TryFrom<ProxyBody> for hudsucker::Body {
    type Error = Error;

    fn try_from(body: ProxyBody) -> Result<Self, Self::Error> {
        let bytes = body.bytes()?;
        Ok(hudsucker::Body::from(Full::from(bytes)))
    }
}

impl TryFrom<ProxyBody> for axum::body::Body {
    type Error = Error;

    fn try_from(body: ProxyBody) -> Result<Self, Self::Error> {
        let bytes = body.bytes()?;
        Ok(axum::body::Body::from(bytes))
    }
}

impl TryFrom<ProxyBody> for Value {
    type Error = Error;

    fn try_from(body: ProxyBody) -> Result<Self, Self::Error> {
        let data = body.to_map()?;

        Ok(serde_json::to_value(data).unwrap_or_default())
    }
}
