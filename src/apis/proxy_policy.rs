use std::collections::BTreeMap;
use k8s_openapi::{
    api::core::v1::ObjectReference,
    apimachinery::pkg::apis::meta::v1::Condition,
};
use kube::CustomResource;
use regorus::Value;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use log::info;
use schemars::gen::SchemaGenerator;
use schemars::schema::Schema;

const MESSAGE_KEY: &str = "data.proxy.message";
const DEFAULT_MESSAGE: &str = "policy should contains message variable";
const RESULT_KEY: &str = "data.proxy.allowed";
const POLICY_NAME: &str = "policy.rego";


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
    pub auth: ProxyPolicyAuth,
    pub rules: Vec<ProxyPolicyRule>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, JsonSchema)]
pub struct ProxyPolicyAuth {
    pub method: ProxyPolicyMethod,
    pub secret: ProxyPolicySecret,
}

#[derive(Serialize, Deserialize,Debug, Clone, JsonSchema)]
pub enum ProxyPolicyMethod {
    #[serde(rename(deserialize = "basicAuth", serialize = "basicAuth"))]
    BasicAuth,
    #[serde(rename(deserialize = "bearerToken", serialize = "bearerToken"))]
    BearerToken,
    #[serde(rename(deserialize = "customerHeader", serialize = "customerHeader"))]
    CustomHeader,
    #[serde(rename(deserialize = "query", serialize = "query"))]
    Query,
}

impl Default for ProxyPolicyMethod {
    fn default() -> Self {
        ProxyPolicyMethod::BasicAuth
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ProxyPolicySecret {
    pub reference: Option<ObjectReference>,
    pub raw: Option<BTreeMap<String, String>>,
}

impl JsonSchema for ProxyPolicySecret {
    fn schema_name() -> String {
        return "ProxyPolicySecret".to_owned();
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        serde_json::from_value(serde_json::json!({
            "type": "object",
            "properties": {
                "reference": {
                    "type": "object",
                    "properties": {
                         "namespace":{
                            "type": "string",
                            "description": "secret namespace",
                        },
                        "name":{
                            "type": "string",
                            "description": "secret name",
                        }
                    }
                },
                "raw": {
                    "type": "object",
                    "description":"secret data"
                }
            }
        })).unwrap()
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, JsonSchema)]
pub struct ProxyPolicyRule {
    pub name: String,
    pub validate: OpaValidator,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, JsonSchema)]
pub struct OpaValidator(pub String);

impl ProxyPolicyRule {
    pub fn eval(&self, input: &BTreeMap<Value, Value>) -> Result<bool> {
        let clone = self.clone();

        let mut engine = regorus::Engine::new();
        engine.add_policy(String::from(POLICY_NAME), clone.validate.0)?;
        engine.set_input(Value::from(input.clone()));

        let allowed = engine.eval_bool_query(RESULT_KEY.to_string(),true)?;
        let mut message = engine.eval_rule(MESSAGE_KEY.to_string())?;
        if message == Value::Undefined {
            message = Value::from(DEFAULT_MESSAGE);
        }

        info!("Policy eval result: {}，message: {}",allowed, message);
        Ok(allowed)
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