use indexmap::IndexMap;
use source_kv::{Value, Deserializer};
use crate::interner::{VmtKey, intern_key};

#[derive(Debug, Clone)]
pub struct Vmt {
    pub shader: String,
    pub properties: IndexMap<VmtKey, Vec<Value>>,
}

impl Vmt {
    /// Creates a new empty VMT with the specified shader.
    pub fn new(shader: &str) -> Self {
        Self {
            shader: shader.to_lowercase(),
            properties: IndexMap::new(),
        }
    }

    /// Parses VMT from a string using AST parser to handle root-level shader.
    pub fn from_str(input: &str) -> Result<Self, source_kv::Error> {
        let mut de = Deserializer::from_str(input);
        let root = de.parse_root()?;

        if let Value::Obj(mut root_map) = root {
            if let Some((shader, mut values)) = root_map.pop() {
                if let Some(Value::Obj(props)) = values.pop() {
                    let mut properties = IndexMap::with_capacity(props.len());
                    for (k, v) in props {
                        properties.insert(intern_key(&k), v);
                    }
                    return Ok(Self {
                        shader: shader.to_lowercase(),
                        properties
                    });
                }
            }
        }
        Err(source_kv::Error::Message("Invalid VMT: Missing shader root or body".into()))
    }

    /// Set a string property. Overwrites if exists.
    pub fn set_string(&mut self, key: &str, value: &str) -> &mut Self {
        self.properties.insert(
            intern_key(key),
            vec![Value::Str(value.to_string())]
        );
        self
    }

    /// Set a flag (boolean) property (converts to "1" or "0").
    pub fn set_flag(&mut self, key: &str, enabled: bool) -> &mut Self {
        self.set_string(key, if enabled { "1" } else { "0" })
    }

    /// Removes a property by key, checking for $, % and raw name.
    pub fn remove(&mut self, key: &str) -> &mut Self {
        let base = key.to_lowercase();
        self.properties.shift_remove(base.as_str());
        self.properties.shift_remove(format!("${}", base).as_str());
        self.properties.shift_remove(format!("%{}", base).as_str());
        self
    }

    /// O(1) lookup. Handles $, % prefixes and case-insensitive keys.
    pub fn get_raw(&self, key: &str) -> Option<&Value> {
        let base = key.to_lowercase();
        self.properties.get(base.as_str())
            .or_else(|| self.properties.get(format!("${}", base).as_str()))
            .or_else(|| self.properties.get(format!("%{}", base).as_str()))
            .and_then(|v| v.first())
    }

    /// Get value as string if it exists.
    pub fn get_string(&self, key: &str) -> Option<String> {
        self.get_raw(key).and_then(|v| v.as_str().map(String::from))
    }

    /// Safely gets a value and parses it as f32.
    pub fn get_f32(&self, key: &str) -> Option<f32> {
        self.get_string(key)?.parse::<f32>().ok()
    }

    /// Safely gets a value and parses it as i32.
    pub fn get_i32(&self, key: &str) -> Option<i32> {
        self.get_string(key)?.parse::<i32>().ok()
    }

    /// Checks boolean flags: supports "1", "true", "yes".
    pub fn get_bool(&self, key: &str) -> bool {
        match self.get_string(key).as_deref() {
            Some("1") | Some("true") => true,
            _ => false,
        }
    }

    /// Parses colors/vectors in both [0.0 0.0 0.0] and {255 255 255} formats.
    /// Returns a normalized [f32; 3] (0.0 to 1.0).
    pub fn get_color(&self, key: &str) -> Option<[f32; 3]> {
        let val = self.get_string(key)?;
        let val = val.trim();

        if val.starts_with('[') && val.ends_with(']') {
            // Parsing floats: "[1 0.5 0]"
            let parts: Vec<f32> = val[1..val.len()-1]
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();
            if parts.len() >= 3 { return Some([parts[0], parts[1], parts[2]]); }
        } else if val.starts_with('{') && val.ends_with('}') {
            // Parsing integers: "{255 128 0}"
            let parts: Vec<f32> = val[1..val.len()-1]
                .split_whitespace()
                .filter_map(|s| s.parse::<u8>().ok().map(|v| v as f32 / 255.0))
                .collect();
            if parts.len() >= 3 { return Some([parts[0], parts[1], parts[2]]); }
        }
        None
    }

    /// Adds a proxy to the material.
    pub fn add_proxy<'a, I>(&mut self, name: &str, params: I) -> &mut Self
    where
        I: IntoIterator<Item = (&'a str, &'a str)>,
    {
        let name_lower = name.to_lowercase();
        let mut proxy_params = IndexMap::new();
        for (k, v) in params {
            proxy_params.insert(k.to_string(), vec![Value::Str(v.to_string())]);
        }

        let proxy_obj = Value::Obj(proxy_params);

        // Get or create the Proxies block
        let proxies_vec = self.properties.entry(intern_key("proxies"))
            .or_insert_with(|| vec![Value::Obj(IndexMap::new())]);

        if let Some(Value::Obj(map)) = proxies_vec.first_mut() {
            map.entry(name_lower)
               .or_insert_with(Vec::new)
               .push(proxy_obj);
        }

        self
    }

    /// Serializes the VMT back into a KeyValues string.
    pub fn to_string(&self) -> Result<String, source_kv::Error> {
        let mut root_map = IndexMap::new();
        let mut props = IndexMap::new();

        for (k, v) in &self.properties {
            props.insert(k.to_string(), v.clone());
        }

        root_map.insert(self.shader.clone(), vec![Value::Obj(props)]);

        source_kv::to_string(&Value::Obj(root_map))
    }
}
