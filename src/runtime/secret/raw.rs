use crate::runtime::secret::{SecretData, Storage};
use std::collections::BTreeMap;
use std::sync::Arc;

pub struct Raw(pub Arc<BTreeMap<String, String>>);

impl Storage for Raw {
    fn get(&self) -> SecretData {
        let data = Arc::clone(&self.0);
        Box::pin(async move { Ok(data) })
    }
}
