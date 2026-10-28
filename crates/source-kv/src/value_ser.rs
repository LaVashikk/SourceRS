use serde::{ser, Serialize};

use crate::error::{Error, Result};
use crate::ser::KeySink;
use crate::value::Value;

/// Serializes a Rust value into a `Value` enum.
pub fn to_value<T>(value: &T) -> Result<Value>
where
    T: Serialize,
{
    match value.serialize(ValueSerializer)? {
        Out::One(v) => Ok(v),
        Out::Nothing => Err(Error::Message("Nothing to serialize, the value is None".into())),
        Out::Many(_) => Err(Error::Message("Sequences must be serialized with a key".into())),
    }
}

/// What one field turns into: `None` writes no entry, a sequence writes the key once per element
enum Out {
    Nothing,
    One(Value),
    Many(Vec<Value>),
}

impl Out {
    fn push_into(self, key: &str, entries: &mut Vec<(String, Value)>) {
        match self {
            Out::Nothing => {}
            Out::One(v) => entries.push((key.to_owned(), v)),
            Out::Many(vs) => entries.extend(vs.into_iter().map(|v| (key.to_owned(), v))),
        }
    }
}

fn one(s: impl ToString) -> Result<Out> {
    Ok(Out::One(Value::Str(s.to_string())))
}

struct ValueSerializer;

impl ser::Serializer for ValueSerializer {
    type Ok = Out;
    type Error = Error;

    type SerializeSeq = SeqSerializer;
    type SerializeTuple = ser::Impossible<Out, Error>;
    type SerializeTupleStruct = ser::Impossible<Out, Error>;
    type SerializeTupleVariant = ser::Impossible<Out, Error>;
    type SerializeMap = MapSerializer;
    type SerializeStruct = MapSerializer;
    type SerializeStructVariant = ser::Impossible<Out, Error>;

    fn serialize_bool(self, v: bool) -> Result<Out> {
        one(if v { "1" } else { "0" })
    }

    fn serialize_i8(self, v: i8) -> Result<Out> {
        one(v)
    }

    fn serialize_i16(self, v: i16) -> Result<Out> {
        one(v)
    }

    fn serialize_i32(self, v: i32) -> Result<Out> {
        one(v)
    }

    fn serialize_i64(self, v: i64) -> Result<Out> {
        one(v)
    }

    fn serialize_u8(self, v: u8) -> Result<Out> {
        one(v)
    }

    fn serialize_u16(self, v: u16) -> Result<Out> {
        one(v)
    }

    fn serialize_u32(self, v: u32) -> Result<Out> {
        one(v)
    }

    fn serialize_u64(self, v: u64) -> Result<Out> {
        one(v)
    }

    fn serialize_i128(self, v: i128) -> Result<Out> {
        one(v)
    }

    fn serialize_u128(self, v: u128) -> Result<Out> {
        one(v)
    }

    fn serialize_f32(self, v: f32) -> Result<Out> {
        one(v)
    }

    fn serialize_f64(self, v: f64) -> Result<Out> {
        one(v)
    }

    fn serialize_char(self, v: char) -> Result<Out> {
        one(v)
    }

    fn serialize_str(self, v: &str) -> Result<Out> {
        one(v)
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<Out> {
        Err(Error::Message("Bytes serialization not supported".into()))
    }

    fn serialize_none(self) -> Result<Out> {
        Ok(Out::Nothing)
    }

    fn serialize_some<T>(self, value: &T) -> Result<Out>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    // Writes no entry, same as `to_string` does
    fn serialize_unit(self) -> Result<Out> {
        Ok(Out::Nothing)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Out> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(self, _name: &'static str, _variant_index: u32, variant: &'static str) -> Result<Out> {
        one(variant)
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<Out>
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
    ) -> Result<Out>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Message("Newtype variant serialization not supported".into()))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(SeqSerializer { values: Vec::with_capacity(len.unwrap_or(0)) })
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Err(Error::Message("Tuple serialization not supported".into()))
    }

    fn serialize_tuple_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeTupleStruct> {
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

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(MapSerializer { entries: Vec::with_capacity(len.unwrap_or(0)), key: String::new() })
    }

    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        self.serialize_map(Some(len))
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

struct SeqSerializer {
    values: Vec<Value>,
}

impl ser::SerializeSeq for SeqSerializer {
    type Ok = Out;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        // KeyValues has no nested lists, an inner list just repeats the same key further
        match value.serialize(ValueSerializer)? {
            Out::Nothing => {}
            Out::One(v) => self.values.push(v),
            Out::Many(vs) => self.values.extend(vs),
        }
        Ok(())
    }

    fn end(self) -> Result<Out> {
        Ok(Out::Many(self.values))
    }
}

struct MapSerializer {
    entries: Vec<(String, Value)>,
    key: String,
}

impl ser::SerializeMap for MapSerializer {
    type Ok = Out;
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.key.clear();
        key.serialize(KeySink(&mut self.key))
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(ValueSerializer)?.push_into(&self.key, &mut self.entries);
        Ok(())
    }

    fn end(self) -> Result<Out> {
        Ok(Out::One(Value::Obj(self.entries)))
    }
}

impl ser::SerializeStruct for MapSerializer {
    type Ok = Out;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(ValueSerializer)?.push_into(key, &mut self.entries);
        Ok(())
    }

    fn end(self) -> Result<Out> {
        Ok(Out::One(Value::Obj(self.entries)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    fn s(v: &str) -> Value {
        Value::Str(v.to_string())
    }

    #[test]
    fn test_serialize_primitives() {
        assert_eq!(to_value(&42).unwrap(), s("42"));
        assert_eq!(to_value(&4.14f32).unwrap(), s("4.14"));
        assert_eq!(to_value(&true).unwrap(), s("1"));
        assert_eq!(to_value(&false).unwrap(), s("0"));
        assert_eq!(to_value(&"hello").unwrap(), s("hello"));
    }

    #[test]
    fn test_serialize_simple_struct() {
        #[derive(Serialize)]
        struct Simple {
            name: String,
            value: i32,
        }

        let result = to_value(&Simple { name: "test".to_string(), value: 42 }).unwrap();
        assert_eq!(result, Value::Obj(vec![("name".into(), s("test")), ("value".into(), s("42"))]));
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

        let result = to_value(&Outer { inner: Inner { x: 1.5 }, name: "outer".to_string() }).unwrap();
        assert_eq!(result.as_obj().unwrap().len(), 2);
        assert_eq!(result.get("inner").unwrap().get_string("x"), Some("1.5"));
    }

    #[test]
    fn test_serialize_with_option() {
        #[derive(Serialize)]
        struct WithOption {
            required: String,
            optional: Option<String>,
        }

        let with_some = WithOption { required: "req".to_string(), optional: Some("opt".to_string()) };
        let result = to_value(&with_some).unwrap();
        assert_eq!(result.get_string("optional"), Some("opt"));

        let with_none = WithOption { required: "req".to_string(), optional: None };
        let result = to_value(&with_none).unwrap();
        assert_eq!(result.as_obj().unwrap().len(), 1);
        assert!(result.get("optional").is_none());
    }

    #[test]
    fn unit_writes_nothing() {
        #[derive(Serialize)]
        struct WithUnit {
            a: (),
            b: u32,
        }

        let result = to_value(&WithUnit { a: (), b: 1 }).unwrap();
        assert_eq!(result, Value::Obj(vec![("b".into(), s("1"))]));
    }

    #[test]
    fn vec_field_repeats_its_key() {
        #[derive(Serialize)]
        struct Side {
            id: u32,
        }

        #[derive(Serialize)]
        struct Solid {
            id: u32,
            side: Vec<Side>,
        }

        let result = to_value(&Solid { id: 1, side: vec![Side { id: 2 }, Side { id: 3 }] }).unwrap();
        let ids: Vec<_> = result.get_all("side").map(|v| v.get_string("id").unwrap()).collect();
        assert_eq!(ids, ["2", "3"]);
    }
}
