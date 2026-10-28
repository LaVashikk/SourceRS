use std::borrow::{Borrow, Cow};
use std::collections::HashMap;

use serde::de::{self, DeserializeSeed, IntoDeserializer, MapAccess, SeqAccess, Visitor};
use serde::{forward_to_deserialize_any, Deserialize};

use crate::error::{Error, Result};
use crate::tape::{NodeVal, Tape};
use crate::value::{Value, VALUE_TOKEN};

/// Objects up to this size look for duplicate keys with a plain scan, bigger ones hash once
const SCAN_LIMIT: u32 = 32;
const NONE: u32 = u32::MAX;

/// Deserializes an instance of type `T` from a KeyValues string
///
/// Repeated keys are collected into a `Vec<T>` field, wherever they appear in the object.
/// Struct fields match keys case-insensitively, `&'de str` fields borrow from `s`
pub fn from_str<'de, T: Deserialize<'de>>(s: &'de str) -> Result<T> {
    let tape = Tape::parse(s)?;
    T::deserialize(NodeDe::new(&tape, 0))
}

/// Deserializes an instance of type `T` from a `Value`
///
/// This is useful when you have already parsed a KeyValues structure into a `Value`
/// and want to convert it into a typed Rust structure. Takes the value or a reference to it
pub fn from_value<T: de::DeserializeOwned>(value: impl Borrow<Value>) -> Result<T> {
    let tape = Tape::from_value(value.borrow());
    T::deserialize(NodeDe::new(&tape, 0))
}

fn fold(key: &str) -> Cow<'_, str> {
    if key.bytes().any(|b| b.is_ascii_uppercase()) {
        Cow::Owned(key.to_ascii_lowercase())
    } else {
        Cow::Borrowed(key)
    }
}

/// One node of the tape
#[derive(Clone, Copy)]
struct NodeDe<'t, 'de> {
    tape: &'t Tape<'de>,
    idx: usize,
    /// Objects hand out every entry as is, no grouping of duplicates. Only `Value` asks for it
    raw: bool,
}

impl<'t, 'de> NodeDe<'t, 'de> {
    fn new(tape: &'t Tape<'de>, idx: usize) -> Self {
        NodeDe { tape, idx, raw: false }
    }

    fn str(&self) -> Result<&'de str> {
        match self.tape.nodes[self.idx].val {
            NodeVal::Str(s) => Ok(s),
            NodeVal::Obj { .. } => Err(Error::ExpectedString),
        }
    }
}

macro_rules! parse_num {
    ($($method:ident => $visit:ident($ty:ty);)*) => {$(
        fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
            let s = self.str()?;
            let n = s.parse::<$ty>().map_err(|_| Error::Message(format!("invalid number \"{s}\"")))?;
            visitor.$visit(n)
        }
    )*};
}

impl<'t, 'de> de::Deserializer<'de> for NodeDe<'t, 'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.tape.nodes[self.idx].val {
            NodeVal::Str(s) => visitor.visit_borrowed_str(s),
            NodeVal::Obj { .. } => visitor.visit_map(ObjAccess::new(self, &[])),
        }
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        match self.tape.nodes[self.idx].val {
            NodeVal::Obj { .. } => visitor.visit_map(ObjAccess::new(self, fields)),
            NodeVal::Str(_) => self.deserialize_any(visitor),
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.str()? {
            "1" => visitor.visit_bool(true),
            "0" => visitor.visit_bool(false),
            s => match s.parse::<bool>() {
                Ok(b) => visitor.visit_bool(b),
                Err(_) => Err(Error::Message(format!("Invalid bool: {s}"))),
            },
        }
    }

    parse_num! {
        deserialize_i8 => visit_i8(i8);
        deserialize_i16 => visit_i16(i16);
        deserialize_i32 => visit_i32(i32);
        deserialize_i64 => visit_i64(i64);
        deserialize_i128 => visit_i128(i128);
        deserialize_u8 => visit_u8(u8);
        deserialize_u16 => visit_u16(u16);
        deserialize_u32 => visit_u32(u32);
        deserialize_u64 => visit_u64(u64);
        deserialize_u128 => visit_u128(u128);
        deserialize_f32 => visit_f32(f32);
        deserialize_f64 => visit_f64(f64);
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_some(self)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(self, name: &'static str, visitor: V) -> Result<V::Value> {
        if name == VALUE_TOKEN {
            visitor.visit_newtype_struct(NodeDe { raw: true, ..self })
        } else {
            visitor.visit_newtype_struct(self)
        }
    }

    // A lone value is a list of one, so `Vec<T>` fields work without duplicates too
    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_seq(OneSeq(Some(self)))
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_enum(self.str()?.into_deserializer())
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_unit()
    }

    forward_to_deserialize_any! {
        char str string bytes byte_buf unit unit_struct tuple tuple_struct map identifier
    }
}

struct OneSeq<'t, 'de>(Option<NodeDe<'t, 'de>>);

impl<'t, 'de> SeqAccess<'de> for OneSeq<'t, 'de> {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
        self.0.take().map(|node| seed.deserialize(node)).transpose()
    }
}

/// Chains of same-key children for objects past `SCAN_LIMIT`, by child ordinal
struct Links {
    kids: Vec<u32>,
    next: Vec<u32>,
    dup: Vec<bool>,
}

impl Links {
    fn build(tape: &Tape, idx: usize, len: usize) -> Self {
        let mut links = Links { kids: Vec::with_capacity(len), next: vec![NONE; len], dup: vec![false; len] };
        let mut last: HashMap<Cow<str>, u32> = HashMap::with_capacity(len);
        
        for c in tape.children(idx) {
            let ord = links.kids.len() as u32;
            links.kids.push(c as u32);
            
            if let Some(prev) = last.insert(fold(tape.nodes[c].key), ord) {
                links.next[prev as usize] = ord;
                links.dup[ord as usize] = true;
            }
        }

        links
    }
}

/// Walks an object. Unless `raw`, every key comes out once, at its first position,
/// with all of its values behind it, so derive sees no duplicate fields
struct ObjAccess<'t, 'de> {
    tape: &'t Tape<'de>,
    raw: bool,
    /// Field names of the struct being filled, empty for maps
    fields: &'static [&'static str],
    len: usize,
    first: usize,
    next: usize,
    end: usize,
    links: Option<Links>,
    ord: usize,
    /// (node, ordinal) of the key handed out last, waiting for `next_value`
    pending: Option<(usize, usize)>,
}

impl<'t, 'de> ObjAccess<'t, 'de> {
    fn new(node: NodeDe<'t, 'de>, fields: &'static [&'static str]) -> Self {
        let NodeVal::Obj { len, end } = node.tape.nodes[node.idx].val else {
            unreachable!("ObjAccess over a string node")
        };
        
        let links = (!node.raw && len > SCAN_LIMIT).then(|| Links::build(node.tape, node.idx, len as usize));
        
        ObjAccess {
            tape: node.tape,
            raw: node.raw,
            fields,
            len: len as usize,
            first: node.idx + 1,
            next: node.idx + 1,
            end: end as usize,
            links,
            ord: 0,
            pending: None,
        }
    }

    fn seen_before(&self, c: usize) -> bool {
        let key = self.tape.nodes[c].key;
        let mut i = self.first;

        while i < c {
            if self.tape.nodes[i].key.eq_ignore_ascii_case(key) {
                return true;
            }
            i = self.tape.skip(i);
        }
        
        false
    }

    fn next_child(&mut self) -> Option<(usize, usize)> {
        if let Some(links) = &self.links {
            while self.ord < links.kids.len() {
                let ord = self.ord;
                self.ord += 1;
                
                if !links.dup[ord] {
                    return Some((links.kids[ord] as usize, ord));
                }
            }
            return None;
        }

        while self.next < self.end {
            let c = self.next;
            self.next = self.tape.skip(c);
        
            if self.raw || !self.seen_before(c) {
                return Some((c, 0));
            }
        }
        
        None
    }
}

impl<'t, 'de> MapAccess<'de> for ObjAccess<'t, 'de> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>> {
        let Some((c, ord)) = self.next_child() else {
            return Ok(None);
        };

        self.pending = Some((c, ord));
        seed.deserialize(KeyDe { key: self.tape.nodes[c].key, fields: self.fields }).map(Some)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value> {
        let (c, ord) = self.pending.take().ok_or_else(|| Error::Message("next_value called before next_key".into()))?;
        
        if self.raw {
            return seed.deserialize(NodeDe { tape: self.tape, idx: c, raw: true });
        }

        let members = match &self.links {
            Some(l) => Members::Chain { kids: &l.kids, next: &l.next, ord: ord as u32 },
            None => Members::Scan { at: c, end: self.end },
        };
        seed.deserialize(GroupDe(Group { tape: self.tape, key: self.tape.nodes[c].key, members }))
    }

    fn size_hint(&self) -> Option<usize> {
        self.raw.then_some(self.len)
    }
}

#[derive(Clone)]
enum Members<'a> {
    Scan { at: usize, end: usize },
    Chain { kids: &'a [u32], next: &'a [u32], ord: u32 },
}

/// Node indices of every value one key has in an object
#[derive(Clone)]
struct Group<'a, 't, 'de> {
    tape: &'t Tape<'de>,
    key: &'de str,
    members: Members<'a>,
}

impl Iterator for Group<'_, '_, '_> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        match &mut self.members {
            Members::Scan { at, end } => {
                while *at < *end {
                    let c = *at;
                    *at = self.tape.skip(c);
                    if self.tape.nodes[c].key.eq_ignore_ascii_case(self.key) {
                        return Some(c);
                    }
                }
                None
            }
            
            Members::Chain { kids, next, ord } => {
                if *ord == NONE {
                    return None;
                }
                let o = *ord as usize;
                *ord = next[o];
                Some(kids[o] as usize)
            }
        }
    }
}

/// All values of one key: a scalar or a struct when there is one, a sequence when asked or when there are several
struct GroupDe<'a, 't, 'de>(Group<'a, 't, 'de>);

impl<'a, 't, 'de> GroupDe<'a, 't, 'de> {
    fn single(&self) -> Option<NodeDe<'t, 'de>> {
        let mut it = self.0.clone();
        let first = it.next()?;
        it.next().is_none().then(|| NodeDe::new(self.0.tape, first))
    }

    fn expect_single(&self) -> Result<NodeDe<'t, 'de>> {
        self.single()
            .ok_or_else(|| Error::Message(format!("key \"{}\" is repeated, expected a single value", self.0.key)))
    }
}

macro_rules! to_single {
    ($($method:ident $(($($arg:ident: $ty:ty),*))?;)*) => {$(
        fn $method<V: Visitor<'de>>(self, $($($arg: $ty,)*)? visitor: V) -> Result<V::Value> {
            self.expect_single()?.$method($($($arg,)*)? visitor)
        }
    )*};
}

impl<'a, 't, 'de> de::Deserializer<'de> for GroupDe<'a, 't, 'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.single() {
            Some(node) => node.deserialize_any(visitor),
            None => self.deserialize_seq(visitor),
        }
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_seq(GroupSeq(self.0))
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_some(self)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(self, name: &'static str, visitor: V) -> Result<V::Value> {
        if name == VALUE_TOKEN {
            return self.expect_single()?.deserialize_newtype_struct(name, visitor);
        }
        
        match self.single() {
            Some(node) => node.deserialize_newtype_struct(name, visitor),
            None => visitor.visit_newtype_struct(self),
        }
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_unit()
    }

    to_single! {
        deserialize_bool; deserialize_char; deserialize_str; deserialize_string;
        deserialize_bytes; deserialize_byte_buf; deserialize_unit; deserialize_map; deserialize_identifier;
        deserialize_i8; deserialize_i16; deserialize_i32; deserialize_i64; deserialize_i128;
        deserialize_u8; deserialize_u16; deserialize_u32; deserialize_u64; deserialize_u128;
        deserialize_f32; deserialize_f64;
        deserialize_unit_struct(name: &'static str);
        deserialize_tuple_struct(name: &'static str, len: usize);
        deserialize_struct(name: &'static str, fields: &'static [&'static str]);
        deserialize_enum(name: &'static str, variants: &'static [&'static str]);
    }
}

struct GroupSeq<'a, 't, 'de>(Group<'a, 't, 'de>);

impl<'a, 't, 'de> SeqAccess<'de> for GroupSeq<'a, 't, 'de> {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
        self.0.next().map(|c| seed.deserialize(NodeDe::new(self.0.tape, c))).transpose()
    }
}

/// Map keys keep their authored case. Struct fields match case-insensitively: the key is swapped
/// for the field name it matches, so `"$BaseTexture"` fills `$basetexture` and `"gamedir"` fills `GameDir`
struct KeyDe<'de> {
    key: &'de str,
    fields: &'static [&'static str],
}

impl<'de> de::Deserializer<'de> for KeyDe<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.fields.iter().find(|f| f.eq_ignore_ascii_case(self.key)) {
            Some(field) => visitor.visit_str(field),
            None => visitor.visit_borrowed_str(self.key),
        }
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_any(visitor)
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_borrowed_str(self.key)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_borrowed_str(self.key)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(self, _name: &'static str, visitor: V) -> Result<V::Value> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_unit()
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char bytes byte_buf option unit unit_struct
        seq tuple tuple_struct map struct enum
    }
}
