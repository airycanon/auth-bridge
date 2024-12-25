use http::Uri;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct BaseUrl(String);

impl BaseUrl {
    pub fn replace(&self, uri: &Uri) -> String {
        let path = uri.path();
        let path_query = uri.path_and_query().map(|v| v.as_str()).unwrap_or(path);

        format!("{}{}", self.0, path_query)
    }
}

impl PartialEq<Uri> for BaseUrl {
    fn eq(&self, uri: &Uri) -> bool {
        if let Ok(addr_uri) = self.0.parse::<Uri>() {
            println!("base URL: {}, target: {}", addr_uri, uri);
            addr_uri.scheme() == uri.scheme() && addr_uri.host() == uri.host()
        } else {
            false
        }
    }
}
