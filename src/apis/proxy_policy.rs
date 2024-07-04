use std::collections::HashMap;
use k8s_openapi::{
    api::core::v1::ObjectReference,
    apimachinery::pkg::apis::meta::v1::Condition,
};
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};


// A struct with our chosen Kind will be created for us, using the following kube attrs
#[derive(CustomResource, Serialize, Deserialize, Default, Debug, Clone, JsonSchema)]
#[kube(
    group = "auth-bridge.dev",
    version = "v1alpha1",
    kind = "ProxyPolicy",
    namespaced,
    status = "ProxyPolicyStatus",
)]
pub struct ProxyPolicySpec {
    pub secret: ProxyPolicySecret,
    pub rules: Vec<ProxyPolicyRule>,
    pub method: ProxyPolicyMethod,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub enum ProxyPolicyMethod {
    #[serde(rename(deserialize = "basicAuth", serialize = "basicAuth"))]
    BasicAuth { header: String },
    #[serde(rename(deserialize = "bearerToken", serialize = "bearerToken"))]
    BearerToken { header: String },
}

impl Default for ProxyPolicyMethod {
    fn default() -> Self {
        ProxyPolicyMethod::BasicAuth {
            header: String::from("Authorization"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProxyPolicySecret {
    pub reference: Option<ObjectReference>,
    pub data: Option<HashMap<String, String>>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, JsonSchema)]
pub struct ProxyPolicyRule {
    pub opa: String,
    pub query: String,
}

impl Default for ProxyPolicySecret {
    fn default() -> Self {
        ProxyPolicySecret { reference: None, data: None }
    }
}

impl JsonSchema for ProxyPolicySecret {
    fn schema_name() -> String {
        "ProxyPolicySecret".to_owned()
    }
    fn json_schema(_: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        serde_json::from_value(serde_json::json!({
            "type": "object",
            "properties": {
                "reference":{
                    "type": "object",
                    "properties": {
                        "name": {"type": "string","description":"secret name"},
                        "namespace": {"type": "string","description":"secret namespace"},
                    },
                    "required": [
                        "name",
                        "namespace"
                    ]
                },
                "data": {"type": "object", "description":"secret data"}
            }
        })).unwrap()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
pub struct ProxyPolicyStatus {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(schema_with = "conditions")]
    pub conditions: Vec<Condition>,
}

fn conditions(_: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
    serde_json::from_value(serde_json::json!({
        "type": "array",
        "x-kubernetes-list-type": "map",
        "x-kubernetes-list-map-keys": ["type"],
        "items": {
            "type": "object",
            "properties": {
                "lastTransitionTime": { "format": "date-time", "type": "string" },
                "message": { "type": "string" },
                "observedGeneration": { "type": "integer", "format": "int64", "default": 0 },
                "reason": { "type": "string" },
                "status": { "type": "string" },
                "type": { "type": "string" }
            },
            "required": [
                "lastTransitionTime",
                "message",
                "reason",
                "status",
                "type"
            ],
        },
    }))
        .unwrap()
}