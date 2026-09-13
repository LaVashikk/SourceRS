//! Utility functions and macros used throughout the VMF parser.

use crate::{VmfError, VmfResult};
use indexmap::IndexMap;

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

/// Gets a borrowed string slice (`&str`) for a key. Returns error if key not found.
/// Use this when you only need to read/compare the value without taking ownership.
#[inline(always)]
pub(crate) fn get_key_ref<'a>(map: &'a IndexMap<String, String>, key: &str) -> VmfResult<&'a str> {
    map.get(key)
        .map(|s| s.as_str()) // Returns &str for greater flexibility
        .ok_or_else(|| VmfError::InvalidFormat(format!("'{}' key not found", key)))
}

/// Removes a key and returns the owned `String`. Returns error if key not found.
/// Use this when you need the `String` value itself. Modifies the map.
#[inline(always)]
pub(crate) fn take_key_owned(map: &mut IndexMap<String, String>, key: &str) -> VmfResult<String> {
    map.swap_remove(key)
        .ok_or_else(|| VmfError::InvalidFormat(format!("'{}' key not found", key)))
}

/// Removes a key, parses the value, and returns the result. Returns error if key not found or parsing fails.
/// Modifies the map.
#[inline(always)]
pub(crate) fn take_and_parse_key<T>(map: &mut IndexMap<String, String>, key: &str) -> VmfResult<T>
where
    T: std::str::FromStr,
    VmfError: From<(T::Err, String)>,
{
    let value_string = take_key_owned(map, key)?;
    value_string
        .parse::<T>()
        .map_err(|e| VmfError::from((e, key.to_string())))
}

/// Removes a key and returns the owned `String`, or returns a default `String` if not found.
/// Modifies the map if the key exists.
#[inline(always)]
pub(crate) fn take_key_or_default(
    map: &mut IndexMap<String, String>,
    key: &str,
    default: String,
) -> String {
    map.swap_remove(key).unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::VmfError;
    use indexmap::IndexMap;

    // Test for get_key_ref (replaces get_key! without default)
    #[test]
    fn get_key_ref_existing_key() {
        let mut map = IndexMap::new();
        map.insert("test_key".to_string(), "test_value".to_string());

        // get_key_ref returns Result<&str, VmfError>
        let value = get_key_ref(&map, "test_key").unwrap();
        assert_eq!(value, "test_value");
        // Ensure the map is unchanged
        assert!(map.contains_key("test_key"));
    }

    #[test]
    fn get_key_ref_missing_key() {
        let map = IndexMap::<String, String>::new();
        // get_key_ref returns Err on missing key
        let result = get_key_ref(&map, "test_key");
        assert!(matches!(result, Err(VmfError::InvalidFormat(_))));
        if let Err(VmfError::InvalidFormat(msg)) = result {
            assert!(msg.contains("'test_key' key not found"));
        }
    }

    // Test for take_key_owned
    #[test]
    fn take_key_owned_existing_key() {
        let mut map = IndexMap::new();
        map.insert("test_key".to_string(), "test_value".to_string());

        // take_key_owned returns Result<String, VmfError> and removes the key
        let value = take_key_owned(&mut map, "test_key").unwrap();
        assert_eq!(value, "test_value");
        // Ensure the key is removed
        assert!(!map.contains_key("test_key"));
    }

    #[test]
    fn take_key_owned_missing_key() {
        let mut map = IndexMap::<String, String>::new();
        // take_key_owned returns Err on missing key
        let result = take_key_owned(&mut map, "test_key");
        assert!(matches!(result, Err(VmfError::InvalidFormat(_))));
        if let Err(VmfError::InvalidFormat(msg)) = result {
            assert!(msg.contains("'test_key' key not found"));
        }
    }

    // Test for take_key_or_default (replaces get_key! with default)
    #[test]
    fn take_key_or_default_existing_key() {
        let mut map = IndexMap::new();
        map.insert("test_key".to_string(), "test_value".to_string());
        let default_val = "default".to_string();

        // take_key_or_default returns String and removes the key if found
        let value = take_key_or_default(&mut map, "test_key", default_val.clone());
        assert_eq!(value, "test_value");
        // Ensure the key is removed
        assert!(!map.contains_key("test_key"));
    }

    #[test]
    fn take_key_or_default_missing_key() {
        let mut map = IndexMap::<String, String>::new();
        let default_val = "default".to_string();

        // take_key_or_default returns the default String if key is missing
        let value = take_key_or_default(&mut map, "test_key", default_val.clone());
        assert_eq!(value, default_val);
        // Ensure the map is unchanged
        assert!(map.is_empty());
    }

    // Test for take_and_parse_key (replaces parse_hs_key!)
    #[test]
    fn take_and_parse_key_valid_value() {
        let mut map = IndexMap::new();
        map.insert("test_key".to_string(), "123".to_string());

        // take_and_parse_key returns Result<T, VmfError> and removes the key
        let value = take_and_parse_key::<i32>(&mut map, "test_key").unwrap();
        assert_eq!(value, 123);
        // Ensure the key is removed
        assert!(!map.contains_key("test_key"));
    }

    #[test]
    fn take_and_parse_key_missing_key() {
        let mut map = IndexMap::<String, String>::new();
        // take_and_parse_key returns Err(InvalidFormat) on missing key (via take_key_owned)
        let result = take_and_parse_key::<i32>(&mut map, "test_key");
        assert!(matches!(result, Err(VmfError::InvalidFormat(_))));
        if let Err(VmfError::InvalidFormat(msg)) = result {
            assert!(msg.contains("'test_key' key not found"));
        }
    }

    #[test]
    fn take_and_parse_key_invalid_value() {
        let mut map = IndexMap::new();
        map.insert("test_key".to_string(), "abc".to_string());

        // take_and_parse_key returns Err(ParseInt) on invalid value
        let result = take_and_parse_key::<i32>(&mut map, "test_key");
        assert!(matches!(result, Err(VmfError::ParseInt { key, source: _ }) if key == "test_key"));
        // Ensure the key is still removed (take_key_owned succeeded before parse failed)
        assert!(!map.contains_key("test_key"));
    }

    // Tests for To01String remain unchanged
    #[test]
    fn to_01_string_true() {
        assert_eq!(true.to_01_string(), "1");
    }

    #[test]
    fn to_01_string_false() {
        assert_eq!(false.to_01_string(), "0");
    }
}
