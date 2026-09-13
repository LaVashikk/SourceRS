use source_vmt::Vmt;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = r#"
        "VertexLitGeneric"
        {
            "$basetexture" "models/player/scout/scout_red"
            "$surfaceprop" "flesh"
        }
    "#;

    // Parse the VMT string
    let mut vmt = Vmt::from_str(input)?;

    // Now edit the VMT
    vmt.set_flag("$rimlight", true)
       .set_string("$rimlightexponent", "5")
       .remove("$surfaceprop");

    // Serialize back to a VMT-formatted string
    let result = vmt.to_string()?;

    println!("--- Updated VMT ---");
    println!("{}", result);

    Ok(())
}
