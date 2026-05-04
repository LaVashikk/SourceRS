use source_vmt::Vmt;

#[test]
fn test_duplicate_proxy_names() {
    let input = r#"
        "Shader"
        {
            "Proxies"
            {
                "Sine" { "resultVar" "$v1" }
                "Sine" { "resultVar" "$v2" }
            }
        }
    "#;
    let vmt = Vmt::from_str(input).unwrap();
    let proxies = vmt.proxies();
    assert_eq!(proxies.len(), 2);
    assert_eq!(proxies[0].name, "Sine"); // Case preserved
    assert_eq!(proxies[1].name, "Sine");
    assert_eq!(proxies[0].get_param("resultVar").unwrap(), "$v1");
    assert_eq!(proxies[1].get_param("resultVar").unwrap(), "$v2");
}

#[test]
fn test_empty_proxies() {
    let vmt = Vmt::from_str("\"Shader\" { \"Proxies\" {} }").unwrap();
    assert!(vmt.proxies().is_empty());
    
    let vmt_none = Vmt::from_str("\"Shader\" { }").unwrap();
    assert!(vmt_none.proxies().is_empty());
}

#[test]
fn test_add_proxy_multiple_times() {
    let mut vmt = Vmt::new("Shader");
    vmt.add_proxy("Sine", [("var", "$a")]);
    vmt.add_proxy("Sine", [("var", "$b")]);
    
    let p = vmt.proxies();
    assert_eq!(p.len(), 2);
    assert_eq!(p[0].get_param("VAR").unwrap(), "$a");
    assert_eq!(p[1].get_param("var").unwrap(), "$b");
}
