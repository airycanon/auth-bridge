use crate::api::script::Script;
use crate::proxy::injector::{
    BasicAuthInjector, BearerTokenInjector, BodyInjector, HeaderInjector, Injector, QueryInjector,
};
use crate::runtime::script::Input;
use anyhow::{Result, anyhow};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt::Debug;

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

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AuthPosition {
    Header,
    Query,
    Body,
}

impl AuthMethod {
    pub fn injector(
        &self,
        secret_data: &BTreeMap<String, String>,
        script: Option<Script>,
    ) -> Result<Box<dyn Injector>> {
        match self {
            AuthMethod::BasicAuth {} => {
                let username = secret_data
                    .get("username")
                    .ok_or_else(|| anyhow!("missing username in secret data"))?
                    .to_string();
                let password = secret_data
                    .get("password")
                    .ok_or_else(|| anyhow!("missing password in secret data"))?
                    .to_string();

                Ok(Box::new(BasicAuthInjector::new(username, password)))
            }
            AuthMethod::BearerToken {} => {
                let token = secret_data
                    .get("token")
                    .ok_or_else(|| anyhow!("missing token in secret data"))?
                    .to_string();
                Ok(Box::new(BearerTokenInjector::new(token)))
            }
            AuthMethod::Generic { position, key } => {
                let value = secret_data
                    .get(key)
                    .ok_or_else(|| anyhow!("missing value for key {key}"))?
                    .to_string();
                Ok(build_injector(position, key, value))
            }
            AuthMethod::Dynamic {
                position,
                key,
                script: script_name,
            } => {
                let script = script.ok_or_else(|| anyhow!("missing auth script {script_name}"))?;
                let input = Input(
                    secret_data
                        .iter()
                        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                        .collect(),
                );

                let executor = script.spec.engine.get_executor();
                let value = executor.execute(script.spec.source.clone(), &input)?;
                let value = match value {
                    Value::String(value) => value,
                    Value::Number(value) => value.to_string(),
                    Value::Bool(value) => value.to_string(),
                    value => return Err(anyhow!("unsupported auth value {:?}", value)),
                };

                Ok(build_injector(position, key, value))
            }
        }
    }
}

fn build_injector(position: &AuthPosition, key: &str, value: String) -> Box<dyn Injector> {
    match position {
        AuthPosition::Header => Box::new(HeaderInjector::new(key.to_string(), value)),
        AuthPosition::Query => Box::new(QueryInjector::new(key.to_string(), value)),
        AuthPosition::Body => Box::new(BodyInjector::new(key.to_string(), value)),
    }
}
