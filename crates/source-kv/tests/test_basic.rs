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

#[test]
fn test_backslash_keeps_utf8() {
    // A backslash used to trigger a byte-wise slow path that mangled non-ASCII
    // into Latin-1 and dropped a trailing slash.
    let input = "root { \"msg\" \"path C:\\dir\" \"quoted\" \"say \\\"hi\\\"\" }";
    let mut de = Deserializer::from_str(input);
    let root = de.parse_root().unwrap();
    let obj = root.get("root").unwrap();

    assert_eq!(obj.get_string("msg"), Some("path C:\\dir"));
    // An escaped quote must not terminate the string, and is kept verbatim.
    assert_eq!(obj.get_string("quoted"), Some("say \\\"hi\\\""));
}

#[test]
fn test_keys_keep_case_but_match_insensitively() {
    let input = "root { \"bSnapToGrid\" \"1\" }";
    let mut de = Deserializer::from_str(input);
    let root = de.parse_root().unwrap();
    let obj = root.get("root").unwrap();

    // Stored verbatim...
    assert!(obj.as_obj().unwrap().contains_key("bSnapToGrid"));
    // ...but looked up case-insensitively, as the engine does.
    assert_eq!(obj.get_string("bsnaptogrid"), Some("1"));
    assert_eq!(obj.get_string("BSNAPTOGRID"), Some("1"));
}


#[derive(Debug, Deserialize, PartialEq)]
#[serde(default)]
struct PbrParams {
    #[serde(rename = "$mraotexture")]
    mrao: String,
    #[serde(rename = "$uv_scale")]
    uv_scale: f32,
    #[serde(rename = "$metalnessscale")]
    metalness: f32,
}

impl Default for PbrParams {
    fn default() -> Self {
        PbrParams { mrao: String::new(), uv_scale: 1.0, metalness: 1.0 }
    }
}

#[test]
fn struct_fields_match_keys_regardless_of_case() {
    let input = r#"
        "$MraoTexture" "pbr/concrete/concrete_floor_01_MRAO"
        $UV_Scale 4
        $MetalnessScale "0.67"
    "#;
    let p: PbrParams = from_str(input).unwrap();

    assert_eq!(p.mrao, "pbr/concrete/concrete_floor_01_MRAO", "value case must be preserved");
    assert_eq!(p.uv_scale, 4.0);
    assert!((p.metalness - 0.67).abs() < 1e-6);
}

#[test]
fn parsed_keys_keep_their_authored_case() {
    let mut de = Deserializer::from_str(r#""$MraoTexture" "x""#);
    let value: Value = de.parse_root().unwrap();

    let map = value.as_obj().unwrap();
    assert!(map.contains_key("$MraoTexture"), "stored key was rewritten: {:?}", map.keys().collect::<Vec<_>>());
    assert_eq!(value.get_string("$mraotexture"), Some("x"));
}
