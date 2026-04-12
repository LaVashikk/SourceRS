use indexmap::IndexMap;
use serde::de::{self, Visitor, DeserializeSeed, MapAccess, SeqAccess};
use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Str(String),
    Obj(IndexMap<String, Vec<Value>>),
}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }
}

pub struct Deserializer<'de> {
    input: &'de str,
    cursor: usize,
    line: usize,
    column: usize,
}

impl<'de> Deserializer<'de> {
    pub fn from_str(input: &'de str) -> Self {
        Deserializer {
            input,
            cursor: 0,
            line: 1,
            column: 1,
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.cursor..].chars().next()
    }

    fn next_char(&mut self) -> Option<char> {
        let c = self.peek_char()?;
        self.cursor += c.len_utf8();

        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }

        Some(c)
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() {
                self.next_char();
            } else if c == '/' {
                if self.input[self.cursor..].starts_with("//") {
                    while let Some(nc) = self.next_char() {
                        if nc == '\n' {
                            break;
                        }
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    fn parse_string(&mut self) -> Result<String> {
        self.skip_whitespace();
        match self.peek_char() {
            Some('"') => {
                self.next_char();
                let mut s = String::with_capacity(32);
                let mut escaped = false;

                while let Some(c) = self.next_char() {
                    if escaped {
                        s.push('\\');
                        s.push(c);
                        escaped = false;
                    } else if c == '\\' {
                        escaped = true;
                    } else if c == '"' {
                        return Ok(s);
                    } else {
                        s.push(c);
                    }
                }
                Err(self.error("Unexpected end of file while parsing quoted string"))
            }
            Some(c) if !c.is_whitespace() && c != '{' && c != '}' => {
                 // let mut s = String::new();
                 let start = self.cursor;

                 while let Some(ch) = self.peek_char() {
                     if ch.is_whitespace() || ch == '{' || ch == '}' || ch == '"' {
                        break;
                    }
                    self.next_char();
                 }
                 let end = self.cursor;
                 Ok(self.input[start..end].to_string())
            }
            Some(c) => Err(self.error(&format!("Unexpected character: '{}'", c))),
            None => Err(Error::Eof),
        }
    }

    pub fn parse_value(&mut self) -> Result<Value> {
        self.skip_whitespace();
        match self.peek_char() {
            Some('{') => {
                self.next_char();
                let obj = self.parse_map_content()?;
                Ok(Value::Obj(obj))
            }
            _ => {
                let s = self.parse_string()?;
                Ok(Value::Str(s))
            }
        }
    }

    pub fn parse_map_content(&mut self) -> Result<IndexMap<String, Vec<Value>>> {
        let mut map = IndexMap::new();
        loop {
            self.skip_whitespace();
            match self.peek_char() {
                Some('}') => {
                    self.next_char();
                    return Ok(map);
                }
                None => {
                    return Err(self.error("Expected '}', found EOF"));
                }
                _ => {
                    let key = self.parse_string()?;
                    let value = self.parse_value()?;
                    map.entry(key).or_insert_with(Vec::new).push(value);
                }
            }
        }
    }

    pub fn parse_root(&mut self) -> Result<Value> {
        let mut map = IndexMap::new();
        loop {
            self.skip_whitespace();
            if self.peek_char().is_none() {
                break;
            }
            let key = match self.parse_string() {
                Ok(k) => k,
                Err(Error::Eof) => break,
                Err(e) => return Err(e),
            };
            let value = self.parse_value()?;
            map.entry(key).or_insert_with(Vec::new).push(value);
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

pub fn from_str<'a, T>(s: &'a str) -> Result<T>
where
    T: de::DeserializeOwned,
{
    let mut deserializer = Deserializer::from_str(s);
    let root_value = deserializer.parse_root()?;
    let mut value_deserializer = ValueDeserializer { value: root_value };
    T::deserialize(&mut value_deserializer)
}

struct ValueDeserializer {
    value: Value,
}

impl<'a, 'de> de::Deserializer<'de> for &'a mut ValueDeserializer {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::Str(s) => visitor.visit_str(s),
            Value::Obj(_) => visitor.visit_map(ValueMapAccess::new(&self.value)),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
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
         match &self.value {
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
         match &self.value {
            Value::Str(s) => visitor.visit_i32(s.parse().map_err(Error::IntParse)?),
            _ => Err(Error::ExpectedString),
        }
    }
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
         match &self.value { Value::Str(s) => visitor.visit_i8(s.parse().map_err(Error::IntParse)?), _ => Err(Error::ExpectedString) }
    }
    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
         match &self.value { Value::Str(s) => visitor.visit_i16(s.parse().map_err(Error::IntParse)?), _ => Err(Error::ExpectedString) }
    }
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
         match &self.value { Value::Str(s) => visitor.visit_i64(s.parse().map_err(Error::IntParse)?), _ => Err(Error::ExpectedString) }
    }
    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
         match &self.value { Value::Str(s) => visitor.visit_u8(s.parse().map_err(Error::IntParse)?), _ => Err(Error::ExpectedString) }
    }
    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
         match &self.value { Value::Str(s) => visitor.visit_u16(s.parse().map_err(Error::IntParse)?), _ => Err(Error::ExpectedString) }
    }
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
         match &self.value { Value::Str(s) => visitor.visit_u32(s.parse().map_err(Error::IntParse)?), _ => Err(Error::ExpectedString) }
    }
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
         match &self.value { Value::Str(s) => visitor.visit_u64(s.parse().map_err(Error::IntParse)?), _ => Err(Error::ExpectedString) }
    }
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
         match &self.value { Value::Str(s) => visitor.visit_f32(s.parse().map_err(Error::FloatParse)?), _ => Err(Error::ExpectedString) }
    }
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
         match &self.value { Value::Str(s) => visitor.visit_f64(s.parse().map_err(Error::FloatParse)?), _ => Err(Error::ExpectedString) }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::Message("deserialize_seq called on single Value".into()))
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::Obj(_) => visitor.visit_map(ValueMapAccess::new(&self.value)),
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

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::Message("Bytes not supported".into())) }
    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::Message("Bytes not supported".into())) }
    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::Message("Char not supported".into())) }
    fn deserialize_unit_struct<V>(self, _name: &'static str, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::Message("Unit struct not supported".into())) }
    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::Message("Tuple not supported".into())) }
    fn deserialize_tuple_struct<V>(self, _name: &'static str, _len: usize, _visitor: V) -> Result<V::Value> where V: Visitor<'de> { Err(Error::Message("Tuple struct not supported".into())) }
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> { self.deserialize_str(visitor) }
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> { self.deserialize_any(visitor) }
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
            _ => panic!("ValueMapAccess created for non-Obj"),
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
                let mut key_deserializer = KeyDeserializer { key };
                seed.deserialize(&mut key_deserializer).map(Some)
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

impl<'a, 'de> de::Deserializer<'de> for &'a mut KeyDeserializer<'a> {
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
    vec: &'a Vec<Value>,
}

impl<'a, 'de> de::Deserializer<'de> for &'a mut VecValueDeserializer<'a> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.vec.len() == 1 {
            let mut de = ValueDeserializer { value: self.vec[0].clone() };
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
            let mut de = ValueDeserializer { value: self.vec[0].clone() };
            de.deserialize_str(visitor)
        } else {
             Err(Error::Message("Expected single string, found sequence".into()))
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
         if self.vec.len() == 1 { let mut de = ValueDeserializer { value: self.vec[0].clone() }; de.deserialize_bool(visitor) } else { Err(Error::Message("Expected scalar".into())) }
    }
    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
         if self.vec.len() == 1 { let mut de = ValueDeserializer { value: self.vec[0].clone() }; de.deserialize_i32(visitor) } else { Err(Error::Message("Expected scalar".into())) }
    }
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
         if self.vec.len() == 1 { let mut de = ValueDeserializer { value: self.vec[0].clone() }; de.deserialize_f32(visitor) } else { Err(Error::Message("Expected scalar".into())) }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        if self.vec.is_empty() {
             visitor.visit_none()
        } else if self.vec.len() == 1 {
             visitor.visit_some(self)
        } else {
             visitor.visit_some(self)
        }
    }

    fn deserialize_struct<V>(self, name: &'static str, fields: &'static [&'static str], visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        if self.vec.len() == 1 {
             let mut de = ValueDeserializer { value: self.vec[0].clone() };
             de.deserialize_struct(name, fields, visitor)
        } else {
             Err(Error::Message("Expected struct, found sequence".into()))
        }
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        if self.vec.len() == 1 {
             let mut de = ValueDeserializer { value: self.vec[0].clone() };
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
                let mut deserializer = ValueDeserializer { value: value.clone() };
                seed.deserialize(&mut deserializer).map(Some)
            }
            None => Ok(None),
        }
    }
}
