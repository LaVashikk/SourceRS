//! Entity IO: the connections that wire outputs to inputs.
//!
//! A connection is one line inside an entity's `connections` block:
//! `"OnTrigger" "target<S>input<S>params<S>delay<S>fire_limit"`
//! `<S>` is ESC (`\x1b`) in Source and in Hammer, and a comma in (old???) Hammer++.

use indexmap::IndexMap;
use indexmap::map::Entry;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

/// The field separator Source and Hammer use.
const SEP_ESC: char = '\x1b';

/// Separator for multiple connections sharing one output name in a single KeyValues entry
const MULTI_SEP: char = '\r';
/// The field separator used by Hammer++
const SEP_COMMA: char = ',';

/// Prefix of the `instance:<proxy>;<io>` addressing form.
const INSTANCE_PREFIX: &str = "instance:";

/// Represents a connection between entities.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Connection {
    pub output: String,
    pub target: String,
    pub input: String,
    pub params: String,
    pub delay: f32,
    pub fire_limit: i32,
}

impl Connection {
    /// Parses a connection string value.
    ///
    /// # Arguments
    /// * `output` - The name of the output (e.g. "OnTrigger").
    /// * `value` - The value string from VMF (e.g. "target,input,params,0.0,-1").
    pub fn parse(output: String, value: &str) -> Self {
        // ESC delimiter takes precedence over comma when present.
        let sep = if value.contains(SEP_ESC) {
            SEP_ESC
        } else {
            SEP_COMMA
        };

        let mut head = value.splitn(3, sep);
        let target = head.next().unwrap_or("");
        let input = head.next().unwrap_or("");
        let rest = head.next().unwrap_or("");

        // Extract delay and fire_limit from the end; remaining text is assigned to params.
        let mut tail = rest.rsplitn(3, sep);
        let last = tail.next().unwrap_or("");
        let mid = tail.next().unwrap_or("");
        let (params, delay, fire_limit) = match tail.next() {
            Some(params) => (params, mid, last),
            None => (rest, "", ""),
        };

        Connection {
            output,
            target: target.to_string(),
            input: input.to_string(),
            params: params.to_string(),
            delay: delay.parse().unwrap_or(0.0),
            fire_limit: fire_limit.parse().unwrap_or(-1),
        }
    }

    /// Formats the connection value for VMF, always with the ESC separator.
    pub fn to_value_string(&self) -> String {
        format!(
            "{}{sep}{}{sep}{}{sep}{}{sep}{}",
            self.target,
            self.input,
            self.params,
            self.delay,
            self.fire_limit,
            sep = SEP_ESC
        )
    }

    /// Classifies how [`Connection::target`] addresses its destination.
    pub fn target_kind(&self) -> Target<'_> {
        Target::parse(&self.target)
    }

    /// Reads [`Connection::input`] as `instance:<proxy>;<io>`, if it is one.
    pub fn instance_input(&self) -> Option<InstanceIo<'_>> {
        InstanceIo::parse(&self.input)
    }

    /// Reads [`Connection::output`] as `instance:<proxy>;<io>`, if it is one.
    pub fn instance_output(&self) -> Option<InstanceIo<'_>> {
        InstanceIo::parse(&self.output)
    }

    /// Rewrites [`Connection::input`] to address an entity inside an instance.
    pub fn set_instance_input(&mut self, proxy: &str, io: &str) {
        self.input = format!("{INSTANCE_PREFIX}{proxy};{io}");
    }

    /// Rewrites [`Connection::output`] to address an entity inside an instance.
    pub fn set_instance_output(&mut self, proxy: &str, io: &str) {
        self.output = format!("{INSTANCE_PREFIX}{proxy};{io}");
    }

    /// Deserializes a list of [`Connection`]s from a `connections` block key-value map.
    pub fn from_key_values(map: &IndexMap<String, String>) -> Option<Vec<Self>> {
        if map.is_empty() {
            return None;
        }

        let mut result = Vec::with_capacity(map.len() * 2);
        for (key, value) in map {
            for part in value.split(MULTI_SEP) {
                result.push(Connection::parse(key.clone(), part));
            }
        }
        Some(result)
    }

    /// Serializes a list of [`Connection`]s into a `connections` block key-value map.
    pub fn to_key_values(connections: &[Self]) -> IndexMap<String, String> {
        let mut map = IndexMap::with_capacity(connections.len());
        for conn in connections {
            match map.entry(conn.output.clone()) {
                Entry::Occupied(mut slot) => {
                    let joined: &mut String = slot.get_mut();
                    joined.push(MULTI_SEP);
                    joined.push_str(&conn.to_value_string());
                }
                Entry::Vacant(slot) => {
                    slot.insert(conn.to_value_string());
                }
            }
        }
        map
    }
}

/// A `!`-prefixed target the engine resolves at runtime instead of by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Special<'a> {
    /// `!self` - the entity holding the output.
    SelfEnt,
    /// `!activator` - whatever triggered the chain.
    Activator,
    /// `!caller` - the entity that fired the output.
    Caller,
    /// `!player` - the first player.
    Player,
    /// `!pvsplayer` - the player whose PVS contains the caller.
    PvsPlayer,
    /// `!picker` - the entity under the player's crosshair.
    Picker,
    /// Any other unmapped `!` target.
    Other(&'a str),
}

/// How a connection's `target` string addresses its destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target<'a> {
    /// A plain targetname. Not unique: several entities may share one.
    Name(&'a str),
    /// A trailing-`*` prefix match, e.g. `door_*` yields `door_`.
    /// A bare `*` yields an empty prefix and matches every named entity.
    Wildcard(&'a str),
    /// A runtime-resolved `!` target.
    Special(Special<'a>),
}

impl<'a> Target<'a> {
    /// Classifies a raw target string. Borrows, never allocates.
    pub fn parse(target: &'a str) -> Self {
        if let Some(tag) = target.strip_prefix('!') {
            let special = if tag.eq_ignore_ascii_case("self") {
                Special::SelfEnt
            } else if tag.eq_ignore_ascii_case("activator") {
                Special::Activator
            } else if tag.eq_ignore_ascii_case("caller") {
                Special::Caller
            } else if tag.eq_ignore_ascii_case("player") {
                Special::Player
            } else if tag.eq_ignore_ascii_case("pvsplayer") {
                Special::PvsPlayer
            } else if tag.eq_ignore_ascii_case("picker") {
                Special::Picker
            } else {
                Special::Other(tag)
            };
            return Target::Special(special);
        }

        match target.strip_suffix('*') {
            Some(prefix) => Target::Wildcard(prefix),
            None => Target::Name(target),
        }
    }

    /// The plain name, if this target is one. `None` for wildcards and `!` targets.
    pub fn name(&self) -> Option<&'a str> {
        match self {
            Target::Name(name) => Some(name),
            _ => None,
        }
    }
}

/// The `instance:<proxy>;<io>` form, used to address IO across a `func_instance`
/// boundary. Applies to both input and output names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstanceIo<'a> {
    /// Name of the entity inside the instance, or of the io proxy exposing it.
    pub proxy: &'a str,
    /// The real input or output name on that entity.
    pub io: &'a str,
}

impl<'a> InstanceIo<'a> {
    /// Parses `instance:<proxy>;<io>`. Returns `None` for a plain IO name.
    pub fn parse(value: &'a str) -> Option<Self> {
        let head = value.get(..INSTANCE_PREFIX.len())?;
        if !head.eq_ignore_ascii_case(INSTANCE_PREFIX) {
            return None;
        }
        let (proxy, io) = value[INSTANCE_PREFIX.len()..].split_once(';')?;
        Some(InstanceIo { proxy, io })
    }
}

impl std::fmt::Display for InstanceIo<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{INSTANCE_PREFIX}{};{}", self.proxy, self.io)
    }
}
