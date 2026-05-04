use source_vmt::Vmt;
use serde::Deserialize;

#[derive(Deserialize, Debug, PartialEq)]
struct Inner {
    #[serde(rename = "$val")]
    val: f32,
}

#[derive(Deserialize, Debug, PartialEq)]
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
