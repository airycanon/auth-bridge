use std::env;

pub const FORWARD_PROXY_ENV: Env = Env("FORWARD_PROXY");

pub const REVERSE_PROXY_ENV: Env = Env("REVERSE_PROXY");

pub const SYSTEM_NAMESPACE_ENV: Env = Env("SYSTEM_NAMESPACE");

pub struct Env(&'static str);

impl Env {
    pub fn value(&self) -> String {
        env::var(self.0).unwrap_or_else(|_| String::new())
    }
}


