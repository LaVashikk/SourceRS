use source_vmt::Vmt;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Inner {
    #[serde(rename = "$val")]
    val: f32,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Outer {
    #[serde(rename = "$top")]
    top: String,
    #[serde(default)]
    inner: Option<Inner>,
}

#[derive(Deserialize, Debug, PartialEq)]
enum SurfaceProp {
    #[serde(rename = "stone")]
    Stone,
    #[serde(rename = "metal")]
    Metal,
}

#[derive(Deserialize, Debug)]
struct EnumBlock {
    #[serde(rename = "$prop")]
    prop: SurfaceProp,
}

#[test]
fn test_complex_block_deserialization() {
    let input = r#"
        "Shader"
        {
            "Outer"
            {
                "$top" "high"
                "inner" { "$val" "0.5" }
            }
        }
    "#;
    let vmt = Vmt::from_str(input).unwrap();
    let block: Outer = vmt.get_block("outer").unwrap().unwrap();

    assert_eq!(block.top, "high");
    assert_eq!(block.inner.unwrap().val, 0.5);
}

// TODO!!
// #[test]
// fn test_enum_deserialization() {
//     let input = r#"
//         "Shader"
//         {
//             "Block"
//             {
//                 "$prop" "metal"
//             }
//         }
//     "#;
//     let vmt = Vmt::from_str(input).unwrap();
//     // YEAH, now "Enum not supported"!
//     let block: EnumBlock = vmt.get_block("block").unwrap().unwrap();
//     assert_eq!(block.prop, SurfaceProp::Metal);
// }

#[test]
fn test_serde_defaults() {
    let input = r#"
        "Shader"
        {
            "Outer"
            {
                "$top" "only_top"
            }
        }
    "#;
    let vmt = Vmt::from_str(input).unwrap();
    let block: Outer = vmt.get_block("outer").unwrap().unwrap();
    assert!(block.inner.is_none());
}

#[test]
fn test_deserialization_errors() {
    let vmt = Vmt::from_str("\"Shader\" { \"Block\" \"not_an_object\" }").unwrap();
    let result = vmt.get_block::<Outer>("block");
    assert!(result.is_err());
}

#[test]
fn test_set_block_simple() {
    let mut vmt = Vmt::new("VertexLitGeneric");
    
    let block = Outer {
        top: "test_value".to_string(),
        inner: None,
    };
    
    vmt.set_block("testblock", &block).unwrap();
    
    // Verify we can read it back
    let retrieved: Outer = vmt.get_block("testblock").unwrap().unwrap();
    assert_eq!(retrieved.top, "test_value");
    assert!(retrieved.inner.is_none());
}

#[test]
fn test_set_block_with_nested() {
    let mut vmt = Vmt::new("VertexLitGeneric");
    
    let block = Outer {
        top: "outer_value".to_string(),
        inner: Some(Inner { val: 0.75 }),
    };
    
    vmt.set_block("nested", &block).unwrap();
    
    // Verify we can read it back
    let retrieved: Outer = vmt.get_block("nested").unwrap().unwrap();
    assert_eq!(retrieved.top, "outer_value");
    assert_eq!(retrieved.inner.unwrap().val, 0.75);
}

#[test]
fn test_set_block_overwrites() {
    let mut vmt = Vmt::new("VertexLitGeneric");
    
    let block1 = Outer {
        top: "first".to_string(),
        inner: None,
    };
    
    let block2 = Outer {
        top: "second".to_string(),
        inner: Some(Inner { val: 1.0 }),
    };
    
    vmt.set_block("block", &block1).unwrap();
    vmt.set_block("block", &block2).unwrap();
    
    // Should retrieve the second block
    let retrieved: Outer = vmt.get_block("block").unwrap().unwrap();
    assert_eq!(retrieved.top, "second");
    assert_eq!(retrieved.inner.unwrap().val, 1.0);
}

#[test]
fn test_set_block_roundtrip_with_serialization() {
    let mut vmt = Vmt::new("VertexLitGeneric");
    
    let block = Outer {
        top: "roundtrip".to_string(),
        inner: Some(Inner { val: 0.5 }),
    };
    
    vmt.set_block("data", &block).unwrap();
    
    // Serialize to string and parse back
    let serialized = vmt.to_string().unwrap();
    let parsed = Vmt::from_str(&serialized).unwrap();
    
    // Verify the block survived the roundtrip
    let retrieved: Outer = parsed.get_block("data").unwrap().unwrap();
    assert_eq!(retrieved.top, "roundtrip");
    assert_eq!(retrieved.inner.unwrap().val, 0.5);
}

#[test]
fn test_set_block_with_primitives() {
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Primitives {
        #[serde(rename = "$int")]
        int_val: i32,
        #[serde(rename = "$float")]
        float_val: f32,
        #[serde(rename = "$bool")]
        bool_val: bool,
        #[serde(rename = "$string")]
        string_val: String,
    }
    
    let mut vmt = Vmt::new("VertexLitGeneric");
    
    let block = Primitives {
        int_val: 42,
        float_val: 3.14,
        bool_val: true,
        string_val: "test".to_string(),
    };
    
    vmt.set_block("primitives", &block).unwrap();
    
    let retrieved: Primitives = vmt.get_block("primitives").unwrap().unwrap();
    assert_eq!(retrieved.int_val, 42);
    assert_eq!(retrieved.float_val, 3.14);
    assert_eq!(retrieved.bool_val, true);
    assert_eq!(retrieved.string_val, "test");
}
