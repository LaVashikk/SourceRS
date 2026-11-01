use indexmap::IndexMap;
use crate::vmt::Vmt;
use source_kv::Value;

#[derive(Debug, Clone)]
pub struct Proxy {
    pub name: String,
    pub params: IndexMap<String, String>,
}

impl Proxy {
    /// Get a proxy parameter by key, case-insensitively.
    pub fn get_param(&self, key: &str) -> Option<&String> {
        let lower = key.to_lowercase();
        self.params.get(lower.as_str())
    }
}

impl Vmt {
    /// Extracts and flattens all proxies into an ordered list.
    pub fn proxies(&self) -> Vec<Proxy> {
        let mut result = Vec::new();

        if let Some(Value::Obj(proxies)) = self.get_raw("proxies") {
            for (proxy_name, instance) in proxies {
                if let Value::Obj(entries) = instance {
                    let mut params = IndexMap::new();
                    for (k, v) in entries {
                        // A repeated param keeps its first value, like the engine's FindKey
                        if let Value::Str(s) = v {
                            params.entry(k.to_lowercase()).or_insert_with(|| s.clone());
                        }
                    }
                    result.push(Proxy {
                        name: proxy_name.clone(),
                        params
                    });
                }
            }
        }
        result
    }
}
