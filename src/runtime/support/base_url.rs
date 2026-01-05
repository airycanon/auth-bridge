use http::Uri;
use http::uri::InvalidUri;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct BaseUrl(String);

impl BaseUrl {
    pub fn replace(&self, uri: &Uri) -> Result<Uri, InvalidUri> {
        let path = uri.path();
        let path_query = uri.path_and_query().map(|v| v.as_str()).unwrap_or(path);

        Uri::try_from(format!("{}{}", self.0, path_query))
    }
}

impl PartialEq<Uri> for BaseUrl {
    fn eq(&self, uri: &Uri) -> bool {
        match self.0.parse::<Uri>() {
            Ok(addr_uri) => addr_uri.scheme() == uri.scheme() && addr_uri.host() == uri.host(),
            _ => false,
        }
    }
}
