use source_vmt::Vmt;

#[test]
fn test_basic_parsing() {
    let input = r#"
        "LightmappedGeneric"
        {
            "$basetexture" "brick/brickwall001"
            "$surfaceprop" "brick"
        }
    "#;
    let vmt = Vmt::from_str(input).unwrap();
    assert_eq!(vmt.shader, "lightmappedgeneric");
    assert_eq!(vmt.get_string("basetexture").unwrap(), "brick/brickwall001");
    assert_eq!(vmt.get_string("$surfaceprop").unwrap(), "brick");
}

#[test]
fn test_patch_logic() {
    let base_input = r#"
        "VertexLitGeneric"
        {
            "$basetexture" "old/texture"
            "$color" "[1 1 1]"
        }
    "#;
    let patch_input = r#"
        "patch"
        {
            "include" "materials/base.vmt"
            "insert"
            {
                "$basetexture" "new/texture"
            }
            "replace"
            {
                "$color" "[0 0 0]"
            }
        }
    "#;

    let mut vmt = Vmt::from_str(base_input).unwrap();
    let patch = Vmt::from_str(patch_input).unwrap();

    vmt.apply_patch(&patch);

    assert_eq!(vmt.get_string("basetexture").unwrap(), "new/texture");
    assert_eq!(vmt.get_string("color").unwrap(), "[0 0 0]");
    assert!(vmt.get_raw("insert").is_none());
    assert!(vmt.get_raw("include").is_none());
}

#[test]
fn test_proxies() {
    let input = r#"
        "UnlitGeneric"
        {
            "Proxies"
            {
                "TextureScroll"
                {
                    "texturescrollvar" "$basetexturetransform"
                    "texturescrollrate" "0.1"
                }
            }
        }
    "#;
    let vmt = Vmt::from_str(input).unwrap();
    let proxies = vmt.proxies();
    assert_eq!(proxies.len(), 1);
    assert_eq!(proxies[0].name, "TextureScroll");
    assert_eq!(proxies[0].params.get("texturescrollrate").unwrap(), "0.1");
}

#[test]
fn test_custom_block() {
    use serde::Deserialize;
    #[derive(Deserialize)]
    struct MyBlock {
        #[serde(rename = "$color")]
        color: String,
    }

    let input = r#"
        "Shader"
        {
            "CustomBlock"
            {
                "$color" "red"
            }
        }
    "#;
    let vmt = Vmt::from_str(input).unwrap();
    let block = vmt.get_block::<MyBlock>("customblock").unwrap().unwrap();
    assert_eq!(block.color, "red");
}

#[test]
fn test_ergonomic_getters() {
    let input = r#"
        "Shader"
        {
            "$f32" "1.5"
            "$i32" "42"
            "$bool_y" "yes"
            "$bool_1" "1"
            "$bool_n" "0"
            "$color_brackets" "[1.0 0.5 0.0]"
            "$color_braces" "{255 128 0}"
        }
    "#;
    let vmt = Vmt::from_str(input).unwrap();

    assert_eq!(vmt.get_f32("f32"), Some(1.5));
    assert_eq!(vmt.get_i32("i32"), Some(42));
    assert!(vmt.get_bool("bool_y"));
    assert!(vmt.get_bool("bool_1"));
    assert!(!vmt.get_bool("bool_n"));

    assert_eq!(vmt.get_color("color_brackets"), Some([1.0, 0.5, 0.0]));
    assert_eq!(vmt.get_color("color_braces"), Some([1.0, 128.0/255.0, 0.0]));
}

#[test]
fn test_add_proxy() {
    let mut vmt = Vmt::new("VertexLitGeneric");
    vmt.add_proxy("Sine", [
        ("sineperiod", "2.0"),
        ("resultVar", "$alpha")
    ]);

    let proxies = vmt.proxies();
    assert_eq!(proxies.len(), 1);
    assert_eq!(proxies[0].name, "sine");
    assert_eq!(proxies[0].get_param("resultVar").unwrap(), "$alpha");
}
