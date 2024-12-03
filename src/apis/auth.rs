use crate::apis::auth::AuthMethod::{BasicAuth, BearerToken, Dynamic};
use crate::apis::condition::conditions;
use crate::apis::policy::Engine;
use crate::core::auth::injector::{
    BasicAuthInjector, BearerTokenInjector, BodyInjector, HeaderInjector, Injector, QueryInjector,
};
use crate::core::script::input::Input;
use anyhow::anyhow;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter, Result};

const PACKAGE: &str = "data.auth";

pub struct ScriptKey(String);

impl From<String> for ScriptKey {
    fn from(key: String) -> Self {
        let key = format!("{}.{}", PACKAGE, key);
        ScriptKey(key)
    }
}

impl Display for ScriptKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.0)
    }
}

// A struct with our chosen Kind will be created for us, using the following kube attrs
#[derive(CustomResource, Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[kube(
    group = "auth-bridge.dev",
    version = "v1alpha1",
    kind = "Auth",
    namespaced,
    status = "AuthStatus"
)]
pub struct AuthSpec {
    #[serde(flatten)]
    pub method: AuthMethod,
}
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub enum AuthPosition {
    Query,
    Header,
    Body,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AuthMethod {
    BasicAuth,
    BearerToken,
    Dynamic {
        position: AuthPosition,
        key: String,
        script: String,
        engine: Engine,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
pub struct AuthStatus {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(schema_with = "conditions")]
    pub conditions: Vec<Condition>,
}

impl AuthMethod {
    pub fn injector(&self, data: &BTreeMap<String, String>) -> anyhow::Result<Box<dyn Injector>> {
        let injector: Box<dyn Injector> = match self {
            BasicAuth => {
                let username = data.get("username").ok_or(anyhow!("username required"))?;
                let password = data.get("password").ok_or(anyhow!("password required"))?;
                Box::new(BasicAuthInjector::new(username.clone(), password.clone()))
            }
            BearerToken => {
                let token = data.get("token").ok_or(anyhow!("token required"))?;
                Box::new(BearerTokenInjector::new(token.clone()))
            }
            Dynamic {
                position,
                key,
                script,
                engine,
            } => {
                let json_value = data
                    .iter()
                    .map(|(key, value)| (key.clone(), Value::String(value.clone())))
                    .collect::<Input>();

                let executor = engine.get_executor();
                let value = match executor.execute(script.clone(), &json_value)? {
                    Value::String(value) => value,
                    unknown => return Err(anyhow!("unsupported value type:{}", unknown)),
                };

                let injector: Box<dyn Injector> = match position {
                    AuthPosition::Query => Box::new(QueryInjector::new(key.clone(), value)),
                    AuthPosition::Header => Box::new(HeaderInjector::new(key.clone(), value)),
                    AuthPosition::Body => Box::new(BodyInjector::new(key.clone(), value)),
                };
                injector
            }
        };

        Ok(injector)
    }
}
