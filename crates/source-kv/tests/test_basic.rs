use serde::{Deserialize, Serialize};
use source_kv::{from_str, to_string, from_value, Value, Deserializer};
use indexmap::IndexMap;

#[test]
fn test_from_value() {
    let input = r#"
        "key" "value"
        "num" "123"
        "flag" "1"
    "#;
    let mut de = Deserializer::from_str(input);
    let value: Value = de.parse_root().unwrap();
    
    let simple: Simple = from_value(value).unwrap();
    assert_eq!(simple.key, "value");
    assert_eq!(simple.num, 123);
    assert_eq!(simple.flag, true);
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct Simple {
    key: String,
    num: i32,
    #[serde(default)]
    flag: bool,
}

#[test]
fn test_simple_kv() {
    let input = r#"
        "key" "value"
        "num" "123"
        "flag" "1"
    "#;
    let simple: Simple = from_str(input).unwrap();
    assert_eq!(simple.key, "value");
    assert_eq!(simple.num, 123);
    assert_eq!(simple.flag, true);
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct Nested {
    name: String,
    #[serde(rename = "child")]
    children: Vec<Child>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct Child {
    id: i32,
}

#[test]
fn test_nested_vec() {
    let input = r#"
        "name" "parent"
        "child"
        {
            "id" "1"
        }
        "child"
        {
            "id" "2"
        }
    "#;
    
    let nested: Nested = from_str(input).unwrap();
    assert_eq!(nested.name, "parent");
    assert_eq!(nested.children.len(), 2);
    assert_eq!(nested.children[0].id, 1);
    assert_eq!(nested.children[1].id, 2);

    let output = to_string(&nested).unwrap();
    println!("{}", output);
    // Verify output contains repeated keys
    assert!(output.contains("child"));
    assert!(output.contains("\"id\" \"1\""));
    assert!(output.contains("\"id\" \"2\""));
}

#[test]
fn test_unquoted_keys() {
    let input = r#"
        key "value"
        num "123"
    "#;
    // We reuse Simple struct but ignore flag
    #[derive(Deserialize)]
    struct SimpleUnquoted {
        key: String,
        num: i32,
    }
    let simple: SimpleUnquoted = from_str(input).unwrap();
    assert_eq!(simple.key, "value");
    assert_eq!(simple.num, 123);
}

#[test]
fn test_root_map() {
    let input = r#"
        "key" "value"
    "#;
    let map: IndexMap<String, String> = from_str(input).unwrap();
    assert_eq!(map.get("key").unwrap(), "value");
}

