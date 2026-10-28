use serde::{Deserialize, Serialize};
use source_kv::{from_str, to_string, from_value, Value};
use indexmap::IndexMap;

#[test]
fn test_from_value() {
    let input = r#"
        "key" "value"
        "num" "123"
        "flag" "1"
    "#;
    let value: Value = input.parse().unwrap();

    let borrowed: Simple = from_value(&value).unwrap();
    let simple: Simple = from_value(value).unwrap();
    assert_eq!(borrowed, simple);
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
    let input = "root { \"msg\" \"path C:\\dir\\café\" \"exe\" \"bin\\win64\\\" }";
    let root: Value = input.parse().unwrap();
    let obj = root.get("root").unwrap();

    assert_eq!(obj.get_string("msg"), Some("path C:\\dir\\café"));
    // No escapes in KeyValues, a quote right after a backslash still closes the string
    assert_eq!(obj.get_string("exe"), Some("bin\\win64\\"));
}

#[test]
fn test_keys_keep_case_but_match_insensitively() {
    let input = "root { \"bSnapToGrid\" \"1\" }";
    let root: Value = input.parse().unwrap();
    let obj = root.get("root").unwrap();

    // Stored verbatim...
    assert_eq!(obj.as_obj().unwrap()[0].0, "bSnapToGrid");
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
    let value: Value = r#""$MraoTexture" "x""#.parse().unwrap();

    assert_eq!(value.as_obj().unwrap()[0].0, "$MraoTexture", "stored key was rewritten");
    assert_eq!(value.get_string("$mraotexture"), Some("x"));
}

#[derive(Debug, Deserialize, PartialEq)]
struct Solid {
    id: u32,
    #[serde(default)]
    side: Vec<Child>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct World {
    classname: String,
    #[serde(default)]
    solid: Vec<Solid>,
}

#[test]
fn duplicates_are_grouped_wherever_they_are() {
    let input = r#"
        solid { id 1 side { id 10 } }
        classname worldspawn
        Solid { id 2 }
        SOLID { id 3 side { id 30 } side { id 31 } }
    "#;
    let world: World = from_str(input).unwrap();
    assert_eq!(world.classname, "worldspawn");
    let ids: Vec<_> = world.solid.iter().map(|s| s.id).collect();
    assert_eq!(ids, [1, 2, 3]);
    assert_eq!(world.solid[2].side, [Child { id: 30 }, Child { id: 31 }]);
}

#[test]
fn big_objects_group_the_same_as_small_ones() {
    // Past 32 children the duplicates are found through a hash map instead of a scan
    let mut input = String::from("classname worldspawn\n");
    for i in 0..100 {
        input.push_str(&format!("solid {{ id {i} }}\n"));
        input.push_str(&format!("filler_{i} x\n"));
    }
    let world: World = from_str(&input).unwrap();
    assert_eq!(world.solid.len(), 100);
    assert!(world.solid.iter().enumerate().all(|(i, s)| s.id == i as u32));
}

#[test]
fn repeated_scalar_key_is_an_error_for_a_single_field() {
    let err = from_str::<Simple>(r#"key a key b num 1"#).unwrap_err();
    assert!(err.to_string().contains("\"key\""), "{err}");
}

#[test]
fn borrowed_fields() {
    #[derive(Deserialize)]
    struct Borrowed<'a> {
        name: &'a str,
        #[serde(borrow)]
        tags: Vec<&'a str>,
    }
    let input = String::from(r#"name "prop_static" tags a tags b"#);
    let b: Borrowed = from_str(&input).unwrap();
    assert_eq!(b.name, "prop_static");
    assert_eq!(b.tags, ["a", "b"]);
}

#[test]
fn value_through_serde_keeps_file_order() {
    let input = r#"
        "Entity" { "id" "1" }
        "hidden" { "entity" { "id" "2" } }
        "entity" { "id" "3" }
    "#;
    let parsed = input.parse::<Value>().unwrap();
    let through_serde: Value = from_str(input).unwrap();
    assert_eq!(through_serde, parsed);

    let keys: Vec<_> = parsed.as_obj().unwrap().iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(keys, ["Entity", "hidden", "entity"]);
    assert_eq!(parsed.get_all("entity").count(), 2);
}

#[test]
fn value_round_trips_through_text() {
    let input = r#"
        "Games"
        {
            "Portal 2"
            {
                "GameExe" "portal2.exe"
                "path" "a"
                "path" "b"
            }
        }
    "#;
    let value = input.parse::<Value>().unwrap();
    let text = to_string(&value).unwrap();
    assert!(text.contains("\"Portal 2\"\n"), "block names must be quoted:\n{text}");
    assert_eq!(text.parse::<Value>().unwrap(), value);
}

#[test]
fn syntax_error_position() {
    let input = "root\n{\n    a 1\n    b }\n}";
    match input.parse::<Value>() {
        Err(source_kv::Error::Syntax { line, column, .. }) => assert_eq!((line, column), (4, 7)),
        other => panic!("expected a syntax error, got {other:?}"),
    }
    assert!("a { b c".parse::<Value>().is_err());
    assert!("a \"open".parse::<Value>().is_err());
}

#[test]
fn wide_integers_and_options() {
    #[derive(Deserialize)]
    struct Nums {
        big: i64,
        huge: u64,
        maybe: Option<u32>,
        missing: Option<u32>,
    }
    let n: Nums = from_str("big -5000000000 huge 18000000000000000000 maybe 7").unwrap();
    assert_eq!(n.big, -5_000_000_000);
    assert_eq!(n.huge, 18_000_000_000_000_000_000);
    assert_eq!(n.maybe, Some(7));
    assert_eq!(n.missing, None);
}

#[test]
fn map_keys_keep_case_and_enums_read_from_strings() {
    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    enum Shader {
        LightmappedGeneric,
        VertexLitGeneric,
    }
    #[derive(Deserialize)]
    struct Mat {
        shader: Shader,
        params: IndexMap<String, String>,
    }
    let m: Mat = from_str(r#"Shader VertexLitGeneric params { "$BaseTexture" "x" }"#).unwrap();
    assert_eq!(m.shader, Shader::VertexLitGeneric);
    assert_eq!(m.params.get("$BaseTexture").map(String::as_str), Some("x"));
    assert_eq!(source_kv::to_value(&Shader::LightmappedGeneric).unwrap(), Value::Str("LightmappedGeneric".into()));
}

#[test]
fn bom_and_comments() {
    let input = "\u{feff}// header\nkey value // trailing\nnum \"12\"";
    let v = input.parse::<Value>().unwrap();
    assert_eq!(v.get_string("key"), Some("value"));
    assert_eq!(v.get_string("num"), Some("12"));
}

#[test]
fn capitalized_renames_match_any_case() {
    #[derive(Deserialize)]
    struct Game {
        #[serde(rename = "GameDir")]
        game_dir: String,
        #[serde(rename = "Hammer")]
        hammer: Option<Simple>,
    }
    let g: Game = from_str(r#"gamedir "portal2" HAMMER { key k num 1 }"#).unwrap();
    assert_eq!(g.game_dir, "portal2");
    assert_eq!(g.hammer.unwrap().num, 1);
}

#[test]
fn nul_after_the_last_brace_ends_the_file() {
    let v: Value = "\"dialog\" { \"wide\" \"300\" }\0".parse().unwrap();
    assert_eq!(v.get("dialog").unwrap().get_string("wide"), Some("300"));
}

#[test]
fn deep_nesting_is_an_error_not_a_crash() {
    let nested = |n: usize| format!("{}{}", "a{".repeat(n), "}".repeat(n));
    assert!(nested(100).parse::<Value>().is_ok());
    assert!(nested(101).parse::<Value>().is_err());
    assert!(from_str::<Value>(&nested(100_000)).is_err());
}

#[test]
fn bad_number_shows_the_value() {
    let err = from_str::<Simple>("key k num 1.5").unwrap_err();
    assert_eq!(err.to_string(), "invalid number \"1.5\"");
}
