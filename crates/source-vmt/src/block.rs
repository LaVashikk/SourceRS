use serde::de::DeserializeOwned;
use crate::vmt::Vmt;

impl Vmt {
    /// Deserializes a specific block into a Serde-compatible struct.
    pub fn get_block<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, source_kv::Error> {
        if let Some(val) = self.get_raw(key) {
            let parsed: T = source_kv::from_value(val.clone())?;
            return Ok(Some(parsed));
        }
        Ok(None)
    }
}
