use crate::api::condition::conditions;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::runtime::script::engine::{Executor, RegoExecutor};

// A struct with our chosen Kind will be created for us, using the following kube attrs
#[derive(CustomResource, Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[kube(
    group = "auth-bridge.dev",
    version = "v1alpha1",
    kind = "Script",
    namespaced,
    status = "ScriptStatus"
)]
pub struct ScriptSpec {
    pub source: String,
    pub engine: Engine,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
pub struct ScriptStatus {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(schema_with = "conditions")]
    pub conditions: Vec<Condition>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Engine {
    Rego { query: String },
}

impl Engine {
    pub fn get_executor(&self) -> Box<dyn Executor> {
        match self {
            Engine::Rego { query } => Box::new(RegoExecutor::new(query.clone())),
        }
    }
}