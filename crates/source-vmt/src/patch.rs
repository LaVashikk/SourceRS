use crate::vmt::Vmt;
use crate::interner::intern_key;
use source_kv::Value;

impl Vmt {
    /// Applies patch instructions (insert/replace) to this VMT.
    pub fn apply_patch(&mut self, patch: &Vmt) {
        // Handle 'replace'
        if let Some(Value::Obj(replace_map)) = patch.get_raw("replace") {
            for (k, v) in replace_map {
                self.properties.insert(intern_key(k), v.clone());
            }
        }
        
        // Handle 'insert'
        if let Some(Value::Obj(insert_map)) = patch.get_raw("insert") {
            for (k, v) in insert_map {
                self.properties.insert(intern_key(k), v.clone());
            }
        }

        // Cleanup patch-specific keys from the final VMT
        self.properties.shift_remove(&intern_key("insert"));
        self.properties.shift_remove(&intern_key("replace"));
        self.properties.shift_remove(&intern_key("include"));
        self.properties.shift_remove(&intern_key("$include"));
    }
}
