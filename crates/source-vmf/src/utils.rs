//! Utility functions and case-insensitive key extractors for VMF parsing.

use crate::{ParseCtx, REPEATED_KEY_SEP};
use indexmap::IndexMap;
use std::fmt::Display;
use std::str::FromStr;

/// A trait for converting a boolean value to a "0" or "1" string.
pub trait To01String {
    /// Converts the boolean value to a "0" or "1" string.
    ///
    /// # Returns
    ///
    /// "1" if `self` is true, "0" otherwise.
    fn to_01_string(self) -> String;
}

impl To01String for bool {
    fn to_01_string(self) -> String {
        if self {
            "1".to_string()
        } else {
            "0".to_string()
        }
    }
}

/// Finds the index of `key`, matching ASCII case-insensitively.
#[inline]
fn resolve(map: &IndexMap<String, String>, key: &str) -> Option<usize> {
    map.get_index_of(key)
        .or_else(|| map.keys().position(|k| k.eq_ignore_ascii_case(key)))
}

/// Removes `key` case-insensitively and returns its value.
#[inline]
pub(crate) fn take_key(map: &mut IndexMap<String, String>, key: &str) -> Option<String> {
    let idx = resolve(map, key)?;
    map.shift_remove_index(idx).map(|(_, v)| v)
}

/// Warns if a single-valued field contains multiple values joined by [`REPEATED_KEY_SEP`].
#[inline]
fn warn_if_repeated(key: &str, value: &str, ctx: &mut ParseCtx) {
    if value.contains(REPEATED_KEY_SEP) {
        let parts: Vec<&str> = value.split(REPEATED_KEY_SEP).collect();
        ctx.warn(format!(
            "key '{}' is written {} times but only one value is modelled, keeping none of {:?}",
            key,
            parts.len(),
            parts
        ));
    }
}

/// Removes `key` and parses all values separated by [`REPEATED_KEY_SEP`].
pub(crate) fn take_multi_parse<T>(
    map: &mut IndexMap<String, String>,
    key: &str,
    ctx: &mut ParseCtx,
) -> Vec<T>
where
    T: FromStr,
{
    let Some(value) = take_key(map, key) else {
        return Vec::new();
    };

    value
        .split(REPEATED_KEY_SEP)
        .filter_map(|part| match part.parse::<T>() {
            Ok(parsed) => Some(parsed),
            Err(_) => {
                ctx.warn(format!("key '{}' has unparsable value '{}'", key, part));
                None
            }
        })
        .collect()
}

/// Removes `key`, or warns and returns `default` when it is missing.
pub(crate) fn take_str(
    map: &mut IndexMap<String, String>,
    key: &str,
    default: &str,
    ctx: &mut ParseCtx,
) -> String {
    match take_key(map, key) {
        Some(value) => {
            warn_if_repeated(key, &value, ctx);
            value
        }
        None => {
            ctx.warn_missing(key, default);
            default.to_string()
        }
    }
}

/// Removes `key` and parses it, warning and returning `default` when it is
/// missing or malformed.
pub(crate) fn take_parse<T>(
    map: &mut IndexMap<String, String>,
    key: &str,
    default: T,
    ctx: &mut ParseCtx,
) -> T
where
    T: FromStr + Display,
{
    match take_key(map, key) {
        Some(value) => {
            warn_if_repeated(key, &value, ctx);
            value.parse::<T>().unwrap_or_else(|_| {
                ctx.warn_unparsable(key, &value, &default);
                default
            })
        }
        None => {
            ctx.warn_missing(key, &default);
            default
        }
    }
}

/// Removes `key` and reads it as a VMF boolean ("1" is true), warning and
/// returning `default` when it is missing.
pub(crate) fn take_bool(
    map: &mut IndexMap<String, String>,
    key: &str,
    default: bool,
    ctx: &mut ParseCtx,
) -> bool {
    match take_key(map, key) {
        Some(value) => {
            warn_if_repeated(key, &value, ctx);
            value == "1"
        }
        None => {
            ctx.warn_missing(key, if default { "1" } else { "0" });
            default
        }
    }
}

/// Removes `key` and parses it if present, warning only if the value is malformed.
pub(crate) fn take_opt_parse<T>(
    map: &mut IndexMap<String, String>,
    key: &str,
    ctx: &mut ParseCtx,
) -> Option<T>
where
    T: FromStr,
{
    let value = take_key(map, key)?;
    warn_if_repeated(key, &value, ctx);
    match value.parse::<T>() {
        Ok(parsed) => Some(parsed),
        Err(_) => {
            ctx.warn(format!("key '{}' has unparsable value '{}'", key, value));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map_of(pairs: &[(&str, &str)]) -> IndexMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn lookup_ignores_case() {
        for spelling in ["bsnaptogrid", "BSNAPTOGRID", "bSnapToGrid"] {
            let mut map = map_of(&[("bSnapToGrid", "1")]);
            assert_eq!(take_key(&mut map, spelling), Some("1".to_string()));
            assert!(map.is_empty());
        }
    }

    #[test]
    fn take_key_preserves_order_of_the_rest() {
        let mut map = map_of(&[("a", "1"), ("b", "2"), ("c", "3")]);
        take_key(&mut map, "b");
        assert_eq!(map.keys().collect::<Vec<_>>(), ["a", "c"]);
    }

    #[test]
    fn missing_key_defaults_and_warns() {
        let mut ctx = ParseCtx::default();
        ctx.enter("viewsettings");
        let mut map = IndexMap::new();

        assert_eq!(take_str(&mut map, "name", "unnamed", &mut ctx), "unnamed");
        assert_eq!(take_parse::<i32>(&mut map, "id", -1, &mut ctx), -1);
        assert!(take_bool(&mut map, "flag", true, &mut ctx));

        assert_eq!(ctx.warnings.len(), 3);
        assert!(ctx.warnings[0].starts_with("viewsettings: missing 'name'"));
    }

    #[test]
    fn unparsable_value_defaults_and_warns() {
        let mut ctx = ParseCtx::default();
        let mut map = map_of(&[("id", "abc")]);

        assert_eq!(take_parse::<i32>(&mut map, "id", 7, &mut ctx), 7);
        assert!(map.is_empty(), "the bad key must not leak into extras");
        assert!(ctx.warnings[0].contains("unparsable value 'abc'"));
    }

    #[test]
    fn absent_optional_is_silent_but_malformed_is_not() {
        let mut ctx = ParseCtx::default();
        let mut map = map_of(&[("flags", "nope")]);

        assert_eq!(take_opt_parse::<u32>(&mut map, "rotation", &mut ctx), None);
        assert!(ctx.warnings.is_empty());

        assert_eq!(take_opt_parse::<u32>(&mut map, "flags", &mut ctx), None);
        assert_eq!(ctx.warnings.len(), 1);
    }

    #[test]
    fn to_01_string_roundtrip() {
        assert_eq!(true.to_01_string(), "1");
        assert_eq!(false.to_01_string(), "0");
    }
}
