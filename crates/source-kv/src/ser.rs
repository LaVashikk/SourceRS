use serde::{ser, Serialize};
use crate::error::{Error, Result};
use std::io::Write;

pub struct Serializer<W> {
    writer: W,
    last_key: Option<String>,
    indent: usize,
    is_root: bool,
    pretty_out: bool,
}

impl<W: Write> Serializer<W> {
    pub fn new(writer: W) -> Self {
        Serializer {
            writer,
            last_key: None,
            indent: 0,
            is_root: true,
            pretty_out: false,
        }
    }

    fn write_indent(&mut self) -> Result<()> {
        for _ in 0..self.indent {
            self.writer.write_all(b"\t")?;
        }
        Ok(())
    }

    fn write_key(&mut self) -> Result<()> {
        if let Some(key) = self.last_key.clone() {
            self.write_indent()?;
            write!(self.writer, "\"{}\"", key)?;
            self.writer.write_all(b" ")?;
        }
        Ok(())
    }
}

pub fn to_string<T>(value: &T) -> Result<String>
where
    T: Serialize,
{
    let mut writer = Vec::new();
    let mut serializer = Serializer::new(&mut writer);
    value.serialize(&mut serializer)?;
    Ok(String::from_utf8(writer).map_err(|e| Error::Message(e.to_string()))?)
}

impl<'a, W: Write> ser::Serializer for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = SeqSerializer<'a, W>;
    type SerializeTuple = ser::Impossible<(), Error>;
    type SerializeTupleStruct = ser::Impossible<(), Error>;
    type SerializeTupleVariant = ser::Impossible<(), Error>;
    type SerializeMap = MapSerializer<'a, W>;
    type SerializeStruct = StructSerializer<'a, W>;
    type SerializeStructVariant = ser::Impossible<(), Error>;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.serialize_str(if v { "1" } else { "0" })
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_char(self, v: char) -> Result<()> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.write_key()?;
        write!(self.writer, "\"{}\"", v)?;
        self.writer.write_all(b"\n")?;
        self.last_key = None;
        Ok(())
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<()> {
        Err(Error::Message("Bytes serialization not supported".into()))
    }

    fn serialize_none(self) -> Result<()> {
        self.last_key = None;
        Ok(())
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        self.last_key = None;
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        Err(Error::Message("Unit variant serialization not supported".into()))
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<()>
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
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Message("Newtype variant serialization not supported".into()))
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        let key = self.last_key.take().ok_or(Error::ExpectedKey)?;
        Ok(SeqSerializer {
            serializer: self,
            key,
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
        if self.is_root {
            self.is_root = false;
            Ok(MapSerializer {
                serializer: self,
                is_root: true,
            })
        } else {
             let key_clone = self.last_key.clone();
             if let Some(key) = key_clone {
                self.write_indent()?;
                write!(self.writer, "{}", key)?;
                self.writer.write_all(b"\n")?;
            }
            self.write_indent()?;
            self.writer.write_all(b"{\n")?;
            self.indent += 1;
            self.last_key = None;

            Ok(MapSerializer {
                serializer: self,
                is_root: false,
            })
        }
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct> {
        let map = self.serialize_map(Some(len))?;
        Ok(StructSerializer { map })
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

pub struct SeqSerializer<'a, W> {
    serializer: &'a mut Serializer<W>,
    key: String,
}

impl<'a, W: Write> ser::SerializeSeq for SeqSerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.serializer.last_key = Some(self.key.clone());
        value.serialize(&mut *self.serializer)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

pub struct MapSerializer<'a, W> {
    serializer: &'a mut Serializer<W>,
    is_root: bool,
}

impl<'a, W: Write> ser::SerializeMap for MapSerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        struct KeyCollector(String);
        impl<'k> ser::Serializer for &'k mut KeyCollector {
            type Ok = ();
            type Error = Error;
            type SerializeSeq = ser::Impossible<(), Error>;
            type SerializeTuple = ser::Impossible<(), Error>;
            type SerializeTupleStruct = ser::Impossible<(), Error>;
            type SerializeTupleVariant = ser::Impossible<(), Error>;
            type SerializeMap = ser::Impossible<(), Error>;
            type SerializeStruct = ser::Impossible<(), Error>;
            type SerializeStructVariant = ser::Impossible<(), Error>;

            fn serialize_str(self, v: &str) -> Result<()> {
                self.0 = v.to_string();
                Ok(())
            }
             fn serialize_bool(self, v: bool) -> Result<()> { self.serialize_str(if v { "1" } else { "0" }) }
            fn serialize_i8(self, v: i8) -> Result<()> { self.serialize_str(&v.to_string()) }
            fn serialize_i16(self, v: i16) -> Result<()> { self.serialize_str(&v.to_string()) }
            fn serialize_i32(self, v: i32) -> Result<()> { self.serialize_str(&v.to_string()) }
            fn serialize_i64(self, v: i64) -> Result<()> { self.serialize_str(&v.to_string()) }
            fn serialize_u8(self, v: u8) -> Result<()> { self.serialize_str(&v.to_string()) }
            fn serialize_u16(self, v: u16) -> Result<()> { self.serialize_str(&v.to_string()) }
            fn serialize_u32(self, v: u32) -> Result<()> { self.serialize_str(&v.to_string()) }
            fn serialize_u64(self, v: u64) -> Result<()> { self.serialize_str(&v.to_string()) }
            fn serialize_f32(self, v: f32) -> Result<()> { self.serialize_str(&v.to_string()) }
            fn serialize_f64(self, v: f64) -> Result<()> { self.serialize_str(&v.to_string()) }
            fn serialize_char(self, v: char) -> Result<()> { self.serialize_str(&v.to_string()) }
            fn serialize_bytes(self, _v: &[u8]) -> Result<()> { Err(Error::ExpectedString) }
            fn serialize_none(self) -> Result<()> { Err(Error::ExpectedString) }
            fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<()> { value.serialize(self) }
            fn serialize_unit(self) -> Result<()> { Err(Error::ExpectedString) }
            fn serialize_unit_struct(self, _name: &'static str) -> Result<()> { Err(Error::ExpectedString) }
            fn serialize_unit_variant(self, _name: &'static str, _idx: u32, _var: &'static str) -> Result<()> { Err(Error::ExpectedString) }
            fn serialize_newtype_struct<T: ?Sized + Serialize>(self, _name: &'static str, value: &T) -> Result<()> { value.serialize(self) }
            fn serialize_newtype_variant<T: ?Sized + Serialize>(self, _name: &'static str, _idx: u32, _var: &'static str, _val: &T) -> Result<()> { Err(Error::ExpectedString) }
            fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> { Err(Error::ExpectedString) }
            fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> { Err(Error::ExpectedString) }
            fn serialize_tuple_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeTupleStruct> { Err(Error::ExpectedString) }
            fn serialize_tuple_variant(self, _name: &'static str, _idx: u32, _var: &'static str, _len: usize) -> Result<Self::SerializeTupleVariant> { Err(Error::ExpectedString) }
            fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> { Err(Error::ExpectedString) }
            fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> { Err(Error::ExpectedString) }
            fn serialize_struct_variant(self, _name: &'static str, _idx: u32, _var: &'static str, _len: usize) -> Result<Self::SerializeStructVariant> { Err(Error::ExpectedString) }
        }

        let mut collector = KeyCollector(String::new());
        key.serialize(&mut collector)?;
        self.serializer.last_key = Some(collector.0);
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut *self.serializer)
    }

    fn end(self) -> Result<()> {
        if !self.is_root {
            self.serializer.indent -= 1;
            self.serializer.write_indent()?;
            self.serializer.writer.write_all(b"}\n")?;
        }
        Ok(())
    }
}

pub struct StructSerializer<'a, W> {
    map: MapSerializer<'a, W>,
}

impl<'a, W: Write> ser::SerializeStruct for StructSerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        use ser::SerializeMap;
        self.map.serialize_entry(key, value)
    }

    fn end(self) -> Result<()> {
        use ser::SerializeMap;
        self.map.end()
    }
}
