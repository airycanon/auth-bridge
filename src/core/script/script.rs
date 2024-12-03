use crate::script::engine::Engine;
use crate::script::input::Input;
use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::PartialEq;

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, PartialEq)]
pub struct Script {
    source: String,
    engine: Engine,
}

impl Script {
    pub fn execute(&self, input: &Input) -> Result<Value> {

    }
}
