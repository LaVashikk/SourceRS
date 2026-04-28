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

        if let Some(Value::Obj(proxies_map)) = self.get_raw("proxies") {
            for (proxy_name, instances) in proxies_map {
                for instance in instances {
                    if let Value::Obj(params_map) = instance {
                        let mut params = IndexMap::new();
                        for (k, v) in params_map {
                            if let Some(Value::Str(s)) = v.first() {
                                params.insert(k.to_lowercase(), s.clone());
                            }
                        }
                        result.push(Proxy {
                            name: proxy_name.clone(),
                            params
                        });
                    }
                }
            }
        }
        result
    }
}
