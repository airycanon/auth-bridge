use anyhow::Result;
use std::collections::BTreeMap;
use std::pin::Pin;
use std::sync::Arc;

pub mod raw;
pub use raw::Raw;

pub mod kubernetes;

type SecretData =
    Pin<Box<dyn Future<Output = Result<Arc<BTreeMap<String, String>>>> + Send + 'static>>;

pub trait Storage: Send {
    fn get(&self) -> SecretData;
}
