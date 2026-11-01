use source_vmt::Vmt;

#[test]
fn test_deep_inheritance() {
    let mut base = Vmt::new("Base");
    base.set_string("$color", "red");

    let mut layer1 = Vmt::new("patch");
    let replace = vec![("color".to_string(), source_vmt::Value::Str("green".into()))];
    layer1.properties.insert(source_vmt::intern_key("replace"), vec![source_vmt::Value::Obj(replace)]);

    let mut layer2 = Vmt::new("patch");
    let insert = vec![("detail".to_string(), source_vmt::Value::Str("moss".into()))];
    layer2.properties.insert(source_vmt::intern_key("insert"), vec![source_vmt::Value::Obj(insert)]);

    base.apply_patch(&layer1);
    base.apply_patch(&layer2);

    assert_eq!(base.get_string("color").unwrap(), "green");
    assert_eq!(base.get_string("detail").unwrap(), "moss");
    assert!(base.get_raw("insert").is_none());
    assert!(base.get_raw("replace").is_none());
}

#[test]
fn repeated_patch_key_keeps_all_its_values() {
    let mut base = Vmt::new("Base");
    base.set_string("$detail", "old");

    let patch = Vmt::from_str(r#"
        "patch"
        {
            "include" "materials/base.vmt"
            "replace"
            {
                "$detail" "a"
                "$Detail" "b"
            }
        }
    "#).unwrap();
    base.apply_patch(&patch);

    let details: Vec<_> = base.properties[&source_vmt::intern_key("$detail")].iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(details, ["a", "b"]);
}
