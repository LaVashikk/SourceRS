use serde::de::DeserializeOwned;
use serde::Serialize;
use crate::vmt::Vmt;
use crate::interner::intern_key;

impl Vmt {
    /// Deserializes a specific block into a Serde-compatible struct.
    pub fn get_block<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, source_kv::Error> {
        if let Some(val) = self.get_raw(key) {
            let parsed: T = source_kv::from_value(val.clone())?;
            return Ok(Some(parsed));
        }
        Ok(None)
    }

    /// Serializes a Serde-compatible struct and sets it as a block.
    /// Overwrites the block if it already exists.
    pub fn set_block<T: Serialize>(&mut self, key: &str, value: &T) -> Result<&mut Self, source_kv::Error> {
        let serialized_value = source_kv::to_value(value)?;
        
        self.properties.insert(
            intern_key(key),
            vec![serialized_value]
        );
        
        Ok(self)
    }
}
