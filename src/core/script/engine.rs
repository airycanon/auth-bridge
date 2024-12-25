use crate::core::script::input::Input;
use anyhow::{anyhow, Result};
use log::debug;
use regorus::{Engine as RegoEngine, Value as RegoValue};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub trait Executor {
    fn execute(&self, source: String, input: &Input) -> Result<Value>;
}
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, PartialEq)]
pub struct RegoExecutor {
    query: String,
    script_name: &'static str,
}

impl RegoExecutor {
    pub fn new(query: String) -> Self {
        Self {
            query,
            script_name: "script.rego",
        }
    }
}

impl Executor for RegoExecutor {
    fn execute(&self, source: String, input: &Input) -> Result<Value> {
        debug!("scrip input {:?}", input);

        let mut engine = RegoEngine::new();
        engine.add_policy(String::from(self.script_name), source)?;

        let rego_input = RegoValue::from(input.clone());
        engine.set_input(rego_input);

        let results = engine.eval_query(self.query.clone(), true)?;
        if results.result.is_empty() || results.result[0].expressions.is_empty() {
            return Err(anyhow!("No results returned for rego query {}", self.query));
        }

        let value = &results.result[0].expressions[0].value;
        let json_value = serde_json::to_value(value)?;

        Ok(json_value)
    }
}

