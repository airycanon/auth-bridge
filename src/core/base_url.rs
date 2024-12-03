use http::Uri;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct BaseUrl(String);

impl PartialEq<Uri> for BaseUrl {
    fn eq(&self, uri: &Uri) -> bool {
        if let Ok(addr_uri) = self.0.parse::<Uri>() {
            addr_uri.scheme() == uri.scheme() && addr_uri.host() == uri.host()
        } else {
            false
        }
    }
}
