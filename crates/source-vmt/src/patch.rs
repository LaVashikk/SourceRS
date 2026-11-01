use indexmap::IndexMap;

use crate::vmt::Vmt;
use crate::interner::{VmtKey, intern_key};
use source_kv::Value;

impl Vmt {
    /// Applies patch instructions (insert/replace) to this VMT
    pub fn apply_patch(&mut self, patch: &Vmt) {
        // Handle 'replace'
        if let Some(Value::Obj(replace_entries)) = patch.get_raw("replace") {
            self.merge_patch_block(replace_entries);
        }

        // Handle 'insert'
        if let Some(Value::Obj(insert_entries)) = patch.get_raw("insert") {
            self.merge_patch_block(insert_entries);
        }

        // Cleanup patch-specific keys from the final VMT
        self.properties.shift_remove(&intern_key("insert"));
        self.properties.shift_remove(&intern_key("replace"));
        self.properties.shift_remove(&intern_key("include"));
        self.properties.shift_remove(&intern_key("$include"));
    }

    /// Puts every key of a patch block into `properties`, replacing whatever that key had before
    fn merge_patch_block(&mut self, entries: &[(String, Value)]) {
        // Grouped first: `"$detail" "a"` + `"$Detail" "b"` must both survive, not overwrite each other
        let mut grouped: IndexMap<VmtKey, Vec<Value>> = IndexMap::new();
        for (k, v) in entries {
            grouped.entry(intern_key(k)).or_default().push(v.clone());
        }
        for (k, values) in grouped {
            self.properties.insert(k, values);
        }
    }
}
