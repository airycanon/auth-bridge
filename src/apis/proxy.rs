use crate::apis::condition::conditions;
use crate::core::auth::storage::{Kubernetes, Raw, Storage};
use crate::core::base_url::BaseUrl;
use k8s_openapi::{api::core::v1::ObjectReference, apimachinery::pkg::apis::meta::v1::Condition};
use kube::CustomResource;
use schemars::gen::SchemaGenerator;
use schemars::schema::Schema;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
#[serde(rename_all = "kebab-case")]
pub struct ProxyAuth {
    pub name: String,
    #[serde(flatten)]
    pub storage: AuthStorage,
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

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ProxyPolicy {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
pub struct ProxyStatus {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(schema_with = "conditions")]
    pub conditions: Vec<Condition>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum AuthStorage {
    Raw {
        data: Option<BTreeMap<String, String>>,
    },
    Kubernetes {
        #[schemars(schema_with = "secret_ref")]
        secret_ref: Option<ObjectReference>,
    },
}

impl AuthStorage {
    pub fn driver(&self) -> anyhow::Result<Box<dyn Storage>> {
        let storage: Box<dyn Storage> = match self {
            AuthStorage::Raw { data } => Box::new(Raw(data.clone().unwrap_or_default())),
            AuthStorage::Kubernetes { secret_ref } => {
                Box::new(Kubernetes::new(secret_ref.clone().unwrap_or_default()))
            }
        };

        Ok(storage)
    }
}
