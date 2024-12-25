use crate::apis::auth::AuthMethod;
use crate::apis::condition::conditions;
use crate::core::base_url::BaseUrl;
use crate::core::secret::{Kubernetes, Raw, Storage};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;
use kube::CustomResource;
use schemars::gen::SchemaGenerator;
use schemars::schema::Schema;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use k8s_openapi::api::core::v1::SecretReference;

// A struct with our chosen Kind will be created for us, using the following kube attrs
#[derive(CustomResource, Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[kube(
    group = "auth-bridge.dev",
    version = "v1alpha1",
    kind = "Proxy",
    namespaced,
    status = "ProxyStatus"
)]
pub struct ProxySpec {
    pub address: BaseUrl,
    pub auth: ProxyAuth,
    pub policies: Vec<ProxyPolicy>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProxyAuth {
    pub method: AuthMethod,
    pub storage: AuthStorage,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ProxyPolicy {
    pub script: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
pub struct ProxyStatus {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(schema_with = "conditions")]
    pub conditions: Vec<Condition>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum AuthStorage {
    Raw(BTreeMap<String, String>),
    #[schemars(schema_with = "secret_ref")]
    SecretRef(SecretReference)
}

impl AuthStorage {
    pub fn driver(&self) -> anyhow::Result<Box<dyn Storage>> {
        let storage: Box<dyn Storage> = match self {
            AuthStorage::Raw (data) => Box::new(Raw(data.clone())),
            AuthStorage::SecretRef (secret_ref) => {
                Box::new(Kubernetes::new(secret_ref.clone()))
            }
        };

        Ok(storage)
    }
}

fn secret_ref(_: &mut SchemaGenerator) -> Schema {
    serde_json::from_value(serde_json::json!({
        "type": "object",
        "properties": {
             "namespace":{
                "type": "string",
                "description": "auth namespace",
            },
            "name":{
                "type": "string",
                "description": "auth name",
            }
        }
    }))
    .unwrap()
}
