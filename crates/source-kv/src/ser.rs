use std::fmt::{Display, Write as _};
use std::io::Write;

use serde::{ser, Serialize};

use crate::error::{Error, Result};

const TABS: &[u8] = &[b'\t'; 32];

pub struct Serializer<W> {
    writer: W,
    /// Key of the entry being written. One buffer for the whole run, nested maps overwrite it
    key: String,
    has_key: bool,
    indent: usize,
    is_root: bool,
}

impl<W: Write> Serializer<W> {
    pub fn new(writer: W) -> Self {
        Serializer {
            writer,
            key: String::new(),
            has_key: false,
            indent: 0,
            is_root: true,
        }
    }

    fn write_indent(&mut self) -> Result<()> {
        let mut n = self.indent;
        while n > 0 {
            let k = n.min(TABS.len());
            self.writer.write_all(&TABS[..k])?;
            n -= k;
        }

        Ok(())
    }

    /// Writes `"key" "` (or just `"` for a keyless scalar), the caller finishes the value
    fn open_scalar(&mut self) -> Result<()> {
        if self.has_key {
            self.has_key = false;
            self.write_indent()?;
            self.writer.write_all(b"\"")?;
            self.writer.write_all(self.key.as_bytes())?;
            self.writer.write_all(b"\" \"")?;
        } else {
            self.writer.write_all(b"\"")?;
        }
        
        Ok(())
    }

    fn write_display(&mut self, v: impl Display) -> Result<()> {
        self.open_scalar()?;
        writeln!(self.writer, "{v}\"")?;
        Ok(())
    }
}

pub fn to_string<T>(value: &T) -> Result<String>
where
    T: Serialize,
{
    let mut writer = Vec::with_capacity(1024);
    let mut serializer = Serializer::new(&mut writer);
    value.serialize(&mut serializer)?;
    String::from_utf8(writer).map_err(|e| Error::Message(e.to_string()))
}

impl<'a, W: Write> ser::Serializer for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = SeqSerializer<'a, W>;
    type SerializeTuple = ser::Impossible<(), Error>;
    type SerializeTupleStruct = ser::Impossible<(), Error>;
    type SerializeTupleVariant = ser::Impossible<(), Error>;
    type SerializeMap = MapSerializer<'a, W>;
    type SerializeStruct = MapSerializer<'a, W>;
    type SerializeStructVariant = ser::Impossible<(), Error>;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.serialize_str(if v { "1" } else { "0" })
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.write_display(v)
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.write_display(v)
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.write_display(v)
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        self.write_display(v)
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.write_display(v)
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.write_display(v)
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.write_display(v)
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.write_display(v)
    }

    fn serialize_i128(self, v: i128) -> Result<()> {
        self.write_display(v)
    }

    fn serialize_u128(self, v: u128) -> Result<()> {
        self.write_display(v)
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        self.write_display(v)
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        self.write_display(v)
    }

    fn serialize_char(self, v: char) -> Result<()> {
        self.write_display(v)
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.open_scalar()?;
        self.writer.write_all(v.as_bytes())?;
        self.writer.write_all(b"\"\n")?;
        Ok(())
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<()> {
        Err(Error::Message("Bytes serialization not supported".into()))
    }

    fn serialize_none(self) -> Result<()> {
        self.has_key = false;
        Ok(())
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        self.has_key = false;
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(self, _name: &'static str, _variant_index: u32, variant: &'static str) -> Result<()> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
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
        if !self.has_key {
            return Err(Error::ExpectedKey);
        }
        self.has_key = false;
        
        // Elements may be blocks that reuse the key buffer, so the seq keeps its own copy
        let key = std::mem::take(&mut self.key);
        Ok(SeqSerializer { serializer: self, key })
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

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        if self.is_root {
            self.is_root = false;
            return Ok(MapSerializer { serializer: self, is_root: true });
        }

        if self.has_key {
            self.has_key = false;
            self.write_indent()?;
            self.writer.write_all(b"\"")?;
            self.writer.write_all(self.key.as_bytes())?;
            self.writer.write_all(b"\"\n")?;
        }

        self.write_indent()?;
        self.writer.write_all(b"{\n")?;
        self.indent += 1;

        Ok(MapSerializer { serializer: self, is_root: false })
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
        self.serializer.key.clear();
        self.serializer.key.push_str(&self.key);
        self.serializer.has_key = true;
        
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
        self.serializer.key.clear();
        key.serialize(KeySink(&mut self.serializer.key))?;
        self.serializer.has_key = true;
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

impl<'a, W: Write> ser::SerializeStruct for MapSerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.serializer.key.clear();
        self.serializer.key.push_str(key);
        self.serializer.has_key = true;
        value.serialize(&mut *self.serializer)
    }

    fn end(self) -> Result<()> {
        ser::SerializeMap::end(self)
    }
}

/// Appends a map key as text to the buffer it holds
pub(crate) struct KeySink<'a>(pub &'a mut String);

impl KeySink<'_> {
    fn push(self, v: impl Display) -> Result<()> {
        write!(self.0, "{v}").map_err(|e| Error::Message(e.to_string()))
    }
}

impl ser::Serializer for KeySink<'_> {
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
        self.0.push_str(v);
        Ok(())
    }
    fn serialize_bool(self, v: bool) -> Result<()> { self.serialize_str(if v { "1" } else { "0" }) }
    fn serialize_i8(self, v: i8) -> Result<()> { self.push(v) }
    fn serialize_i16(self, v: i16) -> Result<()> { self.push(v) }
    fn serialize_i32(self, v: i32) -> Result<()> { self.push(v) }
    fn serialize_i64(self, v: i64) -> Result<()> { self.push(v) }
    fn serialize_u8(self, v: u8) -> Result<()> { self.push(v) }
    fn serialize_u16(self, v: u16) -> Result<()> { self.push(v) }
    fn serialize_u32(self, v: u32) -> Result<()> { self.push(v) }
    fn serialize_u64(self, v: u64) -> Result<()> { self.push(v) }
    fn serialize_i128(self, v: i128) -> Result<()> { self.push(v) }
    fn serialize_u128(self, v: u128) -> Result<()> { self.push(v) }
    fn serialize_f32(self, v: f32) -> Result<()> { self.push(v) }
    fn serialize_f64(self, v: f64) -> Result<()> { self.push(v) }
    fn serialize_char(self, v: char) -> Result<()> { self.push(v) }
    fn serialize_bytes(self, _v: &[u8]) -> Result<()> { Err(Error::ExpectedString) }
    fn serialize_none(self) -> Result<()> { Err(Error::ExpectedString) }
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<()> { value.serialize(self) }
    fn serialize_unit(self) -> Result<()> { Err(Error::ExpectedString) }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> { Err(Error::ExpectedString) }
    fn serialize_unit_variant(self, _name: &'static str, _idx: u32, var: &'static str) -> Result<()> { self.serialize_str(var) }
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
