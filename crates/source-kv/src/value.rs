use std::fmt;
use std::str::FromStr;

use serde::de::{self, MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::tape::Tape;

/// Newtype name our deserializer recognizes to hand out an object entry by entry,
/// without grouping duplicate keys, so a `Value` keeps the exact file order
pub(crate) const VALUE_TOKEN: &str = "$source_kv::Value";

/// Represents a KeyValues value
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A simple string value
    Str(String),
    /// An object: entries in file order, duplicate keys included
    Obj(Vec<(String, Value)>),
}

impl Value {
    /// Checks if the value is a string
    pub fn is_str(&self) -> bool {
        matches!(self, Value::Str(_))
    }

    /// Checks if the value is an object (block)
    pub fn is_obj(&self) -> bool {
        matches!(self, Value::Obj(_))
    }

    /// Returns the string value if this is a `Value::Str`
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the entries if this is a `Value::Obj`
    pub fn as_obj(&self) -> Option<&[(String, Value)]> {
        match self {
            Value::Obj(entries) => Some(entries),
            _ => None,
        }
    }

    /// Returns the entries mutably if this is a `Value::Obj`
    pub fn as_obj_mut(&mut self) -> Option<&mut Vec<(String, Value)>> {
        match self {
            Value::Obj(entries) => Some(entries),
            _ => None,
        }
    }

    /// Returns the FIRST value with this key, if the current Value is an object.
    /// Keys are matched case-insensitively, as they are in the Source Engine
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.get_all(key).next()
    }

    /// Returns ALL values with this key in file order (VDF allows duplicate keys).
    /// Keys are matched case-insensitively, as they are in the Source Engine
    pub fn get_all(&self, key: &str) -> impl Iterator<Item = &Value> {
        self.as_obj()
            .into_iter()
            .flatten()
            .filter(move |(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v)
    }

    /// A convenient shortcut: descends into an object by key and immediately tries to return a string
    pub fn get_string(&self, key: &str) -> Option<&str> {
        self.get(key)?.as_str()
    }
}

/// Parses a whole document into a nameless `Value::Obj` holding the top-level entries.
/// Entries keep the file order and the authored case of keys
impl FromStr for Value {
    type Err = crate::Error;

    fn from_str(s: &str) -> crate::Result<Value> {
        // Straight off the tape. `from_str::<Value>` gives the same thing through serde, 78 ms vs 46 ms on a 28 MB VMF
        Ok(Tape::parse(s)?.to_value(0))
    }
}

impl Serialize for Value {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Value::Str(s) => serializer.serialize_str(s),
            Value::Obj(entries) => {
                let mut map = serializer.serialize_map(Some(entries.len()))?;
                for (k, v) in entries {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_newtype_struct(VALUE_TOKEN, ValueVisitor)
    }
}

struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a KeyValues string or object")
    }

    fn visit_newtype_struct<D: Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        deserializer.deserialize_any(self)
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Value, E> {
        Ok(Value::Str(v.to_owned()))
    }

    fn visit_string<E: de::Error>(self, v: String) -> Result<Value, E> {
        Ok(Value::Str(v))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut entries = Vec::with_capacity(map.size_hint().unwrap_or(0));
        
        while let Some(entry) = map.next_entry::<String, Value>()? {
            entries.push(entry);
        }

        Ok(Value::Obj(entries))
    }
}
