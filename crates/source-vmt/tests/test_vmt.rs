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
fn test_key_normalization_and_prefixes() {
    let input = r#"
        "Shader"
        {
            "$basetexture" "path/1"
            "color"        "[1 1 1]"
            "%compilenodraw" "1"
        }
    "#;
    let vmt = Vmt::from_str(input).unwrap();
    assert_eq!(vmt.get_string("basetexture").unwrap(), "path/1");
    assert_eq!(vmt.get_string("$basetexture").unwrap(), "path/1");
    assert_eq!(vmt.get_string("color").unwrap(), "[1 1 1]");
    assert_eq!(vmt.get_string("$color").unwrap(), "[1 1 1]");
    assert_eq!(vmt.get_string("compilenodraw").unwrap(), "1");
    assert_eq!(vmt.get_string("%compilenodraw").unwrap(), "1");
    assert_eq!(vmt.get_string("BASETEXTURE").unwrap(), "path/1");
}

#[test]
fn test_ergonomic_getters_edge_cases() {
    let input = r#"
        "Shader"
        {
            "$f_valid"   "1.5"
            "$f_dot_pre" ".5"
            "$f_dot_suf" "1."
            "$f_scientific" "1e-1"

            "$b_1" "1"
            "$b_true" "true"
            "$b_yes" "1"
            "$b_0" "0"

            "$c_vec" "[1.0 0.5 0.1]"
            "$c_bytes" "{255 127 0}"
            "$c_bytes_overflow" "{300 0 0}"
        }
    "#;
    let vmt = Vmt::from_str(input).unwrap();

    assert_eq!(vmt.get_f32("f_valid"), Some(1.5));
    assert_eq!(vmt.get_f32("f_dot_pre"), Some(0.5));
    assert_eq!(vmt.get_f32("f_dot_suf"), Some(1.0));
    assert_eq!(vmt.get_f32("f_scientific"), Some(0.1));

    assert!(vmt.get_bool("b_1"));
    assert!(vmt.get_bool("b_true"));
    assert!(vmt.get_bool("b_yes"));
    assert!(!vmt.get_bool("b_0"));

    assert_eq!(vmt.get_color("c_vec"), Some([1.0, 0.5, 0.1]));
    assert_eq!(vmt.get_color("c_bytes"), Some([1.0, 127.0/255.0, 0.0]));
    assert_eq!(vmt.get_color("c_bytes_overflow"), None);
}

#[test]
fn test_broken_files() {
    assert!(Vmt::from_str("   ").is_err());
    assert!(Vmt::from_str("\"Shader\"").is_err());
    assert!(Vmt::from_str("\"Shader\" { ").is_err());
}

#[test]
#[cfg(feature = "intern_keys")]
fn test_interning_pointers() {
    use std::sync::Arc;
    let vmt1 = Vmt::from_str("\"S\" { \"$key\" \"v\" }").unwrap();
    let vmt2 = Vmt::from_str("\"S\" { \"$key\" \"v\" }").unwrap();

    let k1 = vmt1.properties.keys().next().unwrap();
    let k2 = vmt2.properties.keys().next().unwrap();

    assert!(Arc::ptr_eq(k1, k2), "Keys should point to the same Arc<str>");
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
