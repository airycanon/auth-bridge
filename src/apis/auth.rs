use crate::apis::auth::AuthMethod::{BasicAuth, BearerToken, Dynamic, Generic};
use crate::apis::condition::conditions;
use crate::apis::script::Script;
use crate::core::script::input::Input;
use crate::http::auth::injector::{
    BasicAuthInjector, BearerTokenInjector, BodyInjector, HeaderInjector, Injector, QueryInjector,
};
use anyhow::anyhow;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;
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

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum AuthPosition {
    Query,
    Header,
    Body,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
pub struct AuthStatus {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(schema_with = "conditions")]
    pub conditions: Vec<Condition>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum AuthMethod {
    BasicAuth {},
    BearerToken {},
    Generic {
        position: AuthPosition,
        key: String,
    },
    Dynamic {
        position: AuthPosition,
        key: String,
        script: String,
    },
}

impl AuthMethod {
    pub fn injector(
        &self,
        data: &BTreeMap<String, String>,
        script: Option<Script>,
    ) -> anyhow::Result<Box<dyn Injector>> {
        let injector: Box<dyn Injector> = match self {
            BasicAuth {} => {
                let username = data
                    .get("username")
                    .ok_or(anyhow!("username required in secret"))?;
                let password = data
                    .get("password")
                    .ok_or(anyhow!("password required in secret"))?;
                Box::new(BasicAuthInjector::new(username.clone(), password.clone()))
            }
            BearerToken {} => {
                let token = data.get("token").ok_or(anyhow!("token required"))?;
                Box::new(BearerTokenInjector::new(token.clone()))
            }
            Generic { position, key } => {
                let value = data.get(key).ok_or(anyhow!("{} required in secret", key))?;

                let injector: Box<dyn Injector> = match position {
                    AuthPosition::Query => Box::new(QueryInjector::new(key.clone(), value.clone())),
                    AuthPosition::Header => {
                        Box::new(HeaderInjector::new(key.clone(), value.clone()))
                    }
                    AuthPosition::Body => Box::new(BodyInjector::new(key.clone(), value.clone())),
                };
                injector
            }

            Dynamic {
                position,
                key,
                script: script_name,
            } => {
                let script = script.ok_or(anyhow!("script {} not found", script_name))?;

                let json_value = data
                    .iter()
                    .map(|(key, value)| (key.clone(), Value::String(value.clone())))
                    .collect::<Input>();

                let executor = script.spec.engine.get_executor();
                let value = match executor.execute(script.spec.source, &json_value)? {
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
