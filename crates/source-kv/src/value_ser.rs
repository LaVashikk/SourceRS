use serde::{ser, Serialize};
use indexmap::IndexMap;
use crate::error::{Error, Result};
use crate::de::Value;

/// Serializes a Rust value into a `Value` enum.
pub fn to_value<T>(value: &T) -> Result<Value>
where
    T: Serialize,
{
    value.serialize(ValueSerializer)
}

/// A serializer that converts Rust types into `Value` enum.
struct ValueSerializer;

impl ser::Serializer for ValueSerializer {
    type Ok = Value;
    type Error = Error;

    type SerializeSeq = SeqSerializer;
    type SerializeTuple = ser::Impossible<Value, Error>;
    type SerializeTupleStruct = ser::Impossible<Value, Error>;
    type SerializeTupleVariant = ser::Impossible<Value, Error>;
    type SerializeMap = MapSerializer;
    type SerializeStruct = StructSerializer;
    type SerializeStructVariant = ser::Impossible<Value, Error>;

    fn serialize_bool(self, v: bool) -> Result<Value> {
        Ok(Value::Str(if v { "1" } else { "0" }.to_string()))
    }

    fn serialize_i8(self, v: i8) -> Result<Value> {
        Ok(Value::Str(v.to_string()))
    }

    fn serialize_i16(self, v: i16) -> Result<Value> {
        Ok(Value::Str(v.to_string()))
    }

    fn serialize_i32(self, v: i32) -> Result<Value> {
        Ok(Value::Str(v.to_string()))
    }

    fn serialize_i64(self, v: i64) -> Result<Value> {
        Ok(Value::Str(v.to_string()))
    }

    fn serialize_u8(self, v: u8) -> Result<Value> {
        Ok(Value::Str(v.to_string()))
    }

    fn serialize_u16(self, v: u16) -> Result<Value> {
        Ok(Value::Str(v.to_string()))
    }

    fn serialize_u32(self, v: u32) -> Result<Value> {
        Ok(Value::Str(v.to_string()))
    }

    fn serialize_u64(self, v: u64) -> Result<Value> {
        Ok(Value::Str(v.to_string()))
    }

    fn serialize_f32(self, v: f32) -> Result<Value> {
        Ok(Value::Str(v.to_string()))
    }

    fn serialize_f64(self, v: f64) -> Result<Value> {
        Ok(Value::Str(v.to_string()))
    }

    fn serialize_char(self, v: char) -> Result<Value> {
        Ok(Value::Str(v.to_string()))
    }

    fn serialize_str(self, v: &str) -> Result<Value> {
        Ok(Value::Str(v.to_string()))
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<Value> {
        Err(Error::Message("Bytes serialization not supported".into()))
    }

    fn serialize_none(self) -> Result<Value> {
        // Skip None values - they won't be included in the output
        Err(Error::Message("None values are not supported in KeyValues format".into()))
    }

    fn serialize_some<T>(self, value: &T) -> Result<Value>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Value> {
        Ok(Value::Str(String::new()))
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Value> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Value> {
        Ok(Value::Str(variant.to_string()))
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Value>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Value>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Message("Newtype variant serialization not supported".into()))
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(SeqSerializer {
            values: Vec::new(),
        })
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Err(Error::Message("Tuple serialization not supported".into()))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(Error::Message("Tuple struct serialization not supported".into()))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(Error::Message("Tuple variant serialization not supported".into()))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(MapSerializer {
            map: IndexMap::new(),
            current_key: None,
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct> {
        Ok(StructSerializer {
            map: IndexMap::new(),
        })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(Error::Message("Struct variant serialization not supported".into()))
    }
}

/// Serializer for sequences (arrays).
struct SeqSerializer {
    values: Vec<Value>,
}

impl ser::SerializeSeq for SeqSerializer {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.values.push(value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        // For KeyValues, sequences are represented as multiple values with the same key
        // This will be handled by the parent serializer
        // For now, we return the first value or an error if empty
        if self.values.len() == 1 {
            Ok(self.values.into_iter().next().unwrap())
        } else {
            // Multiple values - this should be handled at a higher level
            Err(Error::Message("Sequences with multiple values must be serialized with a key".into()))
        }
    }
}

/// Serializer for maps.
struct MapSerializer {
    map: IndexMap<String, Vec<Value>>,
    current_key: Option<String>,
}

impl ser::SerializeMap for MapSerializer {
    type Ok = Value;
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let key_value = key.serialize(ValueSerializer)?;
        match key_value {
            Value::Str(s) => {
                self.current_key = Some(s);
                Ok(())
            }
            _ => Err(Error::Message("Map keys must be strings".into())),
        }
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let key = self.current_key.take()
            .ok_or_else(|| Error::Message("serialize_value called without key".into()))?;
        
        let serialized_value = value.serialize(ValueSerializer)?;
        self.map.entry(key)
            .or_insert_with(Vec::new)
            .push(serialized_value);
        
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Obj(self.map))
    }
}

/// Serializer for structs.
struct StructSerializer {
    map: IndexMap<String, Vec<Value>>,
}

impl ser::SerializeStruct for StructSerializer {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        // Skip None values (Option<T> fields)
        let serialized_value = match value.serialize(ValueSerializer) {
            Ok(v) => v,
            Err(e) if e.to_string().contains("None values") => return Ok(()),
            Err(e) => return Err(e),
        };
        
        self.map.entry(key.to_string())
            .or_insert_with(Vec::new)
            .push(serialized_value);
        
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Obj(self.map))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[test]
    fn test_serialize_primitives() {
        assert_eq!(to_value(&42).unwrap(), Value::Str("42".to_string()));
        assert_eq!(to_value(&3.14f32).unwrap(), Value::Str("3.14".to_string()));
        assert_eq!(to_value(&true).unwrap(), Value::Str("1".to_string()));
        assert_eq!(to_value(&false).unwrap(), Value::Str("0".to_string()));
        assert_eq!(to_value(&"hello").unwrap(), Value::Str("hello".to_string()));
    }

    #[test]
    fn test_serialize_simple_struct() {
        #[derive(Serialize)]
        struct Simple {
            name: String,
            value: i32,
        }

        let s = Simple {
            name: "test".to_string(),
            value: 42,
        };

        let result = to_value(&s).unwrap();
        
        if let Value::Obj(map) = result {
            assert_eq!(map.len(), 2);
            assert_eq!(map.get("name").unwrap()[0], Value::Str("test".to_string()));
            assert_eq!(map.get("value").unwrap()[0], Value::Str("42".to_string()));
        } else {
            panic!("Expected Value::Obj");
        }
    }

    #[test]
    fn test_serialize_nested_struct() {
        #[derive(Serialize)]
        struct Inner {
            x: f32,
        }

        #[derive(Serialize)]
        struct Outer {
            inner: Inner,
            name: String,
        }

        let outer = Outer {
            inner: Inner { x: 1.5 },
            name: "outer".to_string(),
        };

        let result = to_value(&outer).unwrap();
        
        if let Value::Obj(map) = result {
            assert_eq!(map.len(), 2);
            
            if let Value::Obj(inner_map) = &map.get("inner").unwrap()[0] {
                assert_eq!(inner_map.get("x").unwrap()[0], Value::Str("1.5".to_string()));
            } else {
                panic!("Expected nested Value::Obj");
            }
        } else {
            panic!("Expected Value::Obj");
        }
    }

    #[test]
    fn test_serialize_with_option() {
        #[derive(Serialize)]
        struct WithOption {
            required: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            optional: Option<String>,
        }

        let with_some = WithOption {
            required: "req".to_string(),
            optional: Some("opt".to_string()),
        };

        let result = to_value(&with_some).unwrap();
        if let Value::Obj(map) = result {
            assert_eq!(map.len(), 2);
            assert!(map.contains_key("optional"));
        } else {
            panic!("Expected Value::Obj");
        }

        let with_none = WithOption {
            required: "req".to_string(),
            optional: None,
        };

        let result = to_value(&with_none).unwrap();
        if let Value::Obj(map) = result {
            assert_eq!(map.len(), 1);
            assert!(!map.contains_key("optional"));
        } else {
            panic!("Expected Value::Obj");
        }
    }
}
