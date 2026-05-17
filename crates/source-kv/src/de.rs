use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde::de::{self, Visitor, DeserializeSeed, MapAccess, SeqAccess};
use crate::error::{Error, Result};

/// Represents a KeyValues value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    /// A simple string value.
    Str(String),
    /// An object containing multiple KeyValues entries.
    /// Uses `IndexMap` to preserve order and `Vec` to handle duplicate keys.
    Obj(IndexMap<String, Vec<Value>>),
}

impl Value {
    /// Checks if the value is a string.
    pub fn is_str(&self) -> bool {
        matches!(self, Value::Str(_))
    }

    /// Checks if the value is an object (block).
    pub fn is_obj(&self) -> bool {
        matches!(self, Value::Obj(_))
    }

    /// Returns the string value if this is a `Value::Str`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Returns a reference to the internal IndexMap if this is a `Value::Obj`.
    pub fn as_obj(&self) -> Option<&IndexMap<String, Vec<Value>>> {
        match self {
            Value::Obj(map) => Some(map),
            _ => None,
        }
    }

    /// Returns a mutable reference to the internal IndexMap if this is a `Value::Obj`.
    pub fn as_obj_mut(&mut self) -> Option<&mut IndexMap<String, Vec<Value>>> {
        match self {
            Value::Obj(map) => Some(map),
            _ => None,
        }
    }

    /// A convenient method to get the FIRST value by key, if the current Value is an object.
    /// Automatically accounts for case-insensitive keys in Source Engine (it is recommended to pass the key in lowercase).
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.as_obj()?.get(key)?.first()
    }

    /// Returns ALL values by key (since VDF allows duplicate keys).
    pub fn get_all(&self, key: &str) -> Option<&Vec<Value>> {
        self.as_obj()?.get(key)
    }

    /// A convenient shortcut: descends into an object by key and immediately tries to return a string.
    pub fn get_string(&self, key: &str) -> Option<&str> {
        self.get(key)?.as_str()
    }
}

/// A parser for KeyValues (VDF) format.
pub struct Deserializer<'de> {
    input: &'de str,
    cursor: usize,
    line: usize,
    column: usize,
}

impl<'de> Deserializer<'de> {
    /// Creates a new deserializer from a string.
    pub fn from_str(input: &'de str) -> Self {
        Deserializer {
            input,
            cursor: 0,
            line: 1,
            column: 1,
        }
    }

    #[inline]
    fn peek_byte(&self) -> Option<u8> {
        self.input.as_bytes().get(self.cursor).copied()
    }

    fn skip_whitespace(&mut self) {
        let bytes = self.input.as_bytes();
        while self.cursor < bytes.len() {
            let b = bytes[self.cursor];
            if b.is_ascii_whitespace() {
                if b == b'\n' {
                    self.line += 1;
                    self.column = 1;
                } else {
                    self.column += 1;
                }
                self.cursor += 1;
            } else if b == b'/' && self.cursor + 1 < bytes.len() && bytes[self.cursor + 1] == b'/' {
                self.cursor += 2;
                while self.cursor < bytes.len() && bytes[self.cursor] != b'\n' {
                    self.cursor += 1;
                }
            } else {
                break;
            }
        }
    }

    fn parse_string(&mut self) -> Result<String> {
        self.skip_whitespace();
        let bytes = self.input.as_bytes();
        if self.cursor >= bytes.len() {
            return Err(Error::Eof);
        }

        if bytes[self.cursor] == b'"' {
            self.cursor += 1;
            self.column += 1;
            let start = self.cursor;
            let mut has_escapes = false;

            while self.cursor < bytes.len() {
                let b = bytes[self.cursor];
                if b == b'"' {
                    let end = self.cursor;
                    self.cursor += 1;
                    self.column += 1;
                    if !has_escapes {
                        return Ok(self.input[start..end].to_string());
                    } else {
                        // Slow path for escapes
                        let mut s = String::with_capacity(end - start);
                        let mut esc = false;
                        for &byte in &bytes[start..end] {
                            if esc {
                                s.push('\\');
                                s.push(byte as char);
                                esc = false;
                            } else if byte == b'\\' {
                                esc = true;
                            } else {
                                s.push(byte as char);
                            }
                        }
                        return Ok(s);
                    }
                } else if b == b'\\' {
                    has_escapes = true;
                    self.cursor += 2;
                    self.column += 2;
                } else {
                    if b == b'\n' {
                        self.line += 1;
                        self.column = 1;
                    } else {
                        self.column += 1;
                    }
                    self.cursor += 1;
                }
            }
            Err(self.error("Unexpected end of file while parsing quoted string"))
        } else {
            let start = self.cursor;
            while self.cursor < bytes.len() {
                let b = bytes[self.cursor];
                if b.is_ascii_whitespace() || b == b'{' || b == b'}' || b == b'"' {
                    break;
                }
                self.cursor += 1;
                self.column += 1;
            }
            if start == self.cursor {
                return Err(self.error("Expected string"));
            }
            Ok(self.input[start..self.cursor].to_string())
        }
    }

    /// Parses a single KeyValues value.
    pub fn parse_value(&mut self) -> Result<Value> {
        self.skip_whitespace();
        match self.peek_byte() {
            Some(b'{') => {
                self.cursor += 1;
                self.column += 1;
                let obj = self.parse_map_content()?;
                Ok(Value::Obj(obj))
            }
            Some(b'"') => {
                let s = self.parse_string()?;
                Ok(Value::Str(s))
            }
            Some(b) => {
                if b != b'}' {
                    let s = self.parse_string()?;
                    Ok(Value::Str(s))
                } else {
                    Err(self.error(&format!("Expected '{{' or '\"', found '{}'", b as char)))
                }
            }
            None => Err(Error::Eof),
        }
    }

    /// Parses the content of a KeyValues object.
    pub fn parse_map_content(&mut self) -> Result<IndexMap<String, Vec<Value>>> {
        let mut map = IndexMap::with_capacity(8);
        loop {
            self.skip_whitespace();
            match self.peek_byte() {
                Some(b'}') => {
                    self.cursor += 1;
                    self.column += 1;
                    return Ok(map);
                }
                None => return Err(self.error("Expected '}', found EOF")),
                _ => {
                    let key = self.parse_string()?.to_lowercase();
                    let value = self.parse_value()?;
                    map.entry(key).or_insert_with(|| Vec::with_capacity(1)).push(value);
                }
            }
        }
    }

    /// Parses the root level of a KeyValues document.
    pub fn parse_root(&mut self) -> Result<Value> {
        let mut map = IndexMap::with_capacity(16);
        loop {
            self.skip_whitespace();
            if self.peek_byte().is_none() {
                break;
            }
            let key = match self.parse_string() {
                Ok(k) => k,
                Err(Error::Eof) => break,
                Err(e) => return Err(e),
            };
            let value = self.parse_value()?;
            map.entry(key).or_insert_with(|| Vec::with_capacity(1)).push(value);
        }
        Ok(Value::Obj(map))
    }

    fn error(&self, msg: &str) -> Error {
        Error::Syntax {
            line: self.line,
            column: self.column,
            msg: msg.to_string(),
        }
    }
}

/// Deserializes an instance of type `T` from a KeyValues string.
pub fn from_str<'a, T>(s: &'a str) -> Result<T>
where
    T: de::DeserializeOwned,
{
    let mut deserializer = Deserializer::from_str(s);
    let root_value = deserializer.parse_root()?;
    from_value(root_value)
}

/// Deserializes an instance of type `T` from a `Value`.
///
/// This is useful when you have already parsed a KeyValues structure into a `Value`
/// and want to convert it into a typed Rust structure.
pub fn from_value<T>(value: Value) -> Result<T>
where
    T: de::DeserializeOwned,
{
    let mut deserializer = ValueDeserializer { value: &value };
    T::deserialize(&mut deserializer)
}

struct ValueDeserializer<'a> {
    value: &'a Value,
}

impl<'a, 'de> de::Deserializer<'de> for &'a mut ValueDeserializer<'a> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Str(s) => visitor.visit_str(s),
            Value::Obj(_) => visitor.visit_map(ValueMapAccess::new(self.value)),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Str(s) => visitor.visit_str(s),
            _ => Err(Error::ExpectedString),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
         match self.value {
            Value::Str(s) => {
                if s == "1" { visitor.visit_bool(true) }
                else if s == "0" { visitor.visit_bool(false) }
                else {
                    match s.parse::<bool>() {
                        Ok(b) => visitor.visit_bool(b),
                        Err(_) => Err(Error::Message(format!("Invalid bool: {}", s))),
                    }
                }
            },
            _ => Err(Error::ExpectedString),
        }
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
         match self.value {
            Value::Str(s) => visitor.visit_i32(s.parse().map_err(Error::IntParse)?),
            _ => Err(Error::ExpectedString),
        }
    }
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
         match self.value { Value::Str(s) => visitor.visit_i8(s.parse().map_err(Error::IntParse)?), _ => Err(Error::ExpectedString) }
    }
    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
         match self.value { Value::Str(s) => visitor.visit_i16(s.parse().map_err(Error::IntParse)?), _ => Err(Error::ExpectedString) }
    }
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
         match self.value { Value::Str(s) => visitor.visit_i64(s.parse().map_err(Error::IntParse)?), _ => Err(Error::ExpectedString) }
    }
    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
         match self.value { Value::Str(s) => visitor.visit_u8(s.parse().map_err(Error::IntParse)?), _ => Err(Error::ExpectedString) }
    }
    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
         match self.value { Value::Str(s) => visitor.visit_u16(s.parse().map_err(Error::IntParse)?), _ => Err(Error::ExpectedString) }
    }
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
         match self.value { Value::Str(s) => visitor.visit_u32(s.parse().map_err(Error::IntParse)?), _ => Err(Error::ExpectedString) }
    }
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
         match self.value { Value::Str(s) => visitor.visit_u64(s.parse().map_err(Error::IntParse)?), _ => Err(Error::ExpectedString) }
    }
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
         match self.value { Value::Str(s) => visitor.visit_f32(s.parse().map_err(Error::FloatParse)?), _ => Err(Error::ExpectedString) }
    }
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
         match self.value { Value::Str(s) => visitor.visit_f64(s.parse().map_err(Error::FloatParse)?), _ => Err(Error::ExpectedString) }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        match self.value {
            Value::Obj(_) => visitor.visit_seq(ValueSeqAccess { iter: std::slice::from_ref(self.value).iter() }),
            _ => Err(Error::ExpectedObjectStart),
        }
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Obj(_) => visitor.visit_map(ValueMapAccess::new(self.value)),
            _ => Err(Error::ExpectedObjectStart),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::Message("Enum deserialization not supported".into()))
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        match self.value {
            Value::Str(s) => visitor.visit_str(s),
            _ => Err(Error::ExpectedString),
        }
    }
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> { self.deserialize_any(visitor) }

    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::Message("Char not supported".into())) }
    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::Message("Bytes not supported".into())) }
    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::Message("Bytes not supported".into())) }
    fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::Message("Unit not supported".into())) }
    fn deserialize_unit_struct<V>(self, _name: &'static str, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::Message("Unit struct not supported".into())) }
    fn deserialize_newtype_struct<V>(self, _name: &'static str, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::Message("Newtype struct not supported".into())) }
    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::Message("Tuple not supported".into())) }
    fn deserialize_tuple_struct<V>(self, _name: &'static str, _len: usize, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::Message("Tuple struct not supported".into())) }
}

struct ValueMapAccess<'a> {
    iter: indexmap::map::Iter<'a, String, Vec<Value>>,
    next_value: Option<&'a Vec<Value>>,
}

impl<'a> ValueMapAccess<'a> {
    fn new(value: &'a Value) -> Self {
        match value {
            Value::Obj(map) => ValueMapAccess {
                iter: map.iter(),
                next_value: None,
            },
            _ => unreachable!(),
        }
    }
}

impl<'a, 'de> MapAccess<'de> for ValueMapAccess<'a> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.next_value = Some(value);
                let key_deserializer = KeyDeserializer { key };
                seed.deserialize(key_deserializer).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        match self.next_value.take() {
            Some(vec) => {
                let mut deserializer = VecValueDeserializer { vec };
                seed.deserialize(&mut deserializer)
            }
            None => Err(Error::Message("next_value called before next_key".into())),
        }
    }
}

struct KeyDeserializer<'a> {
    key: &'a str,
}

impl<'a, 'de> de::Deserializer<'de> for KeyDeserializer<'a> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> { visitor.visit_str(self.key) }
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> { visitor.visit_str(self.key) }
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> { visitor.visit_str(self.key) }
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> { visitor.visit_str(self.key) }
    fn deserialize_bool<V>(self, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::ExpectedKey) }
    fn deserialize_i8<V>(self, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::ExpectedKey) }
    fn deserialize_i16<V>(self, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::ExpectedKey) }
    fn deserialize_i32<V>(self, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::ExpectedKey) }
    fn deserialize_i64<V>(self, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::ExpectedKey) }
    fn deserialize_u8<V>(self, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::ExpectedKey) }
    fn deserialize_u16<V>(self, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::ExpectedKey) }
    fn deserialize_u32<V>(self, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::ExpectedKey) }
    fn deserialize_u64<V>(self, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::ExpectedKey) }
    fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::ExpectedKey) }
    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::ExpectedKey) }
    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::ExpectedKey) }
    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::ExpectedKey) }
    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::ExpectedKey) }
    fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::ExpectedKey) }
    fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::ExpectedKey) }
    fn deserialize_unit_struct<V>(self, _name: &'static str, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::ExpectedKey) }
    fn deserialize_newtype_struct<V>(self, _name: &'static str, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::ExpectedKey) }
    fn deserialize_seq<V>(self, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::ExpectedKey) }
    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::ExpectedKey) }
    fn deserialize_tuple_struct<V>(self, _name: &'static str, _len: usize, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::ExpectedKey) }
    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::ExpectedKey) }
    fn deserialize_struct<V>(self, _name: &'static str, _fields: &'static [&'static str], _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::ExpectedKey) }
    fn deserialize_enum<V>(self, _name: &'static str, _variants: &'static [&'static str], _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::ExpectedKey) }
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> { self.deserialize_any(visitor) }
}


struct VecValueDeserializer<'a> {
    vec: &'a [Value],
}

impl<'a, 'de> de::Deserializer<'de> for &'a mut VecValueDeserializer<'a> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.vec.len() == 1 {
            let mut de = ValueDeserializer { value: &self.vec[0] };
            de.deserialize_any(visitor)
        } else {
            self.deserialize_seq(visitor)
        }
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(ValueSeqAccess { iter: self.vec.iter() })
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        if self.vec.len() == 1 {
            let mut de = ValueDeserializer { value: &self.vec[0] };
            de.deserialize_str(visitor)
        } else {
             Err(Error::Message("Expected single string, found sequence".into()))
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
         if self.vec.len() == 1 { let mut de = ValueDeserializer { value: &self.vec[0] }; de.deserialize_bool(visitor) } else { Err(Error::Message("Expected scalar".into())) }
    }
    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
         if self.vec.len() == 1 { let mut de = ValueDeserializer { value: &self.vec[0] }; de.deserialize_i32(visitor) } else { Err(Error::Message("Expected scalar".into())) }
    }
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
         if self.vec.len() == 1 { let mut de = ValueDeserializer { value: &self.vec[0] }; de.deserialize_f32(visitor) } else { Err(Error::Message("Expected scalar".into())) }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        if self.vec.is_empty() {
             visitor.visit_none()
        } else {
             visitor.visit_some(self)
        }
    }

    fn deserialize_struct<V>(self, name: &'static str, fields: &'static [&'static str], visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        if self.vec.len() == 1 {
             let mut de = ValueDeserializer { value: &self.vec[0] };
             de.deserialize_struct(name, fields, visitor)
        } else {
             Err(Error::Message("Expected struct, found sequence".into()))
        }
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        if self.vec.len() == 1 {
             let mut de = ValueDeserializer { value: &self.vec[0] };
             de.deserialize_map(visitor)
        } else {
             Err(Error::Message("Expected map, found sequence".into()))
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> { self.deserialize_i32(visitor) }
    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> { self.deserialize_i32(visitor) }
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> { self.deserialize_i32(visitor) }
    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> { self.deserialize_i32(visitor) }
    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> { self.deserialize_i32(visitor) }
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> { self.deserialize_i32(visitor) }
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> { self.deserialize_i32(visitor) }
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> { self.deserialize_f32(visitor) }
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> { self.deserialize_str(visitor) }
    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::Message("Char not supported".into())) }
    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::Message("Bytes not supported".into())) }
    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::Message("Bytes not supported".into())) }
    fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::Message("Unit not supported".into())) }
    fn deserialize_unit_struct<V>(self, _name: &'static str, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::Message("Unit struct not supported".into())) }
    fn deserialize_newtype_struct<V>(self, _name: &'static str, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::Message("Newtype struct not supported".into())) }
    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::Message("Tuple not supported".into())) }
    fn deserialize_tuple_struct<V>(self, _name: &'static str, _len: usize, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::Message("Tuple struct not supported".into())) }
    fn deserialize_enum<V>(self, _name: &'static str, _variants: &'static [&'static str], _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::Message("Enum not supported".into())) }
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> { self.deserialize_str(visitor) }
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> { self.deserialize_any(visitor) }

}

struct ValueSeqAccess<'a> {
    iter: std::slice::Iter<'a, Value>,
}

impl<'a, 'de> SeqAccess<'de> for ValueSeqAccess<'a> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => {
                let mut deserializer = ValueDeserializer { value };
                seed.deserialize(&mut deserializer).map(Some)
            }
            None => Ok(None),
        }
    }
}
