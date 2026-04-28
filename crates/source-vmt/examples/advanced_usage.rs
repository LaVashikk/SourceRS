use source_vmt::Vmt;
use serde::Deserialize;

// Custom struct for a block that might exist in specific Source branches
#[derive(Deserialize, Debug)]
struct PbrBlock {
    #[serde(rename = "$bumpmap")]
    bump_map: String,
    #[serde(rename = "$mraotexture")]
    mrao_texture: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = r#"
        "Water"
        {
            "$forceexpensive" "1"
            "$phongboost" "1.5"
            "$color" "[1.0 0.5 0.0]"
            "$envmaptint" "{255 128 0}"

            "PBR"
            {
                "$bumpmap" "liquids/water_river_normal_sharp"
                "$mraotexture" "liquids/water_noise"
            }
        }
    "#;

    let mut vmt = Vmt::from_str(input)?;

    println!("Shader: {}", vmt.shader);

    println!("Phong Boost (f32): {:?}", vmt.get_f32("phongboost"));
    println!("Expensive Water (bool): {}", vmt.get_bool("forceexpensive"));

    if let Some(color) = vmt.get_color("color") {
        println!("Normalized Color [v]: {:?}", color);
    }

    if let Some(tint) = vmt.get_color("envmaptint") {
        println!("Normalized Tint {{v}}: {:?}", tint);
    }

    // Deserialize a custom block into our struct
    if let Some(pbr) = vmt.get_block::<PbrBlock>("PBR")? {
        println!("\n--- Found PBR Block ---");
        println!("BumpMap: {}", pbr.bump_map);
        println!("MRAO: {}", pbr.mrao_texture);
    }

    // And remove the PBR block
    vmt.remove("PBR");

    // Add a proxy
    vmt.add_proxy("Sine", [
        ("sineperiod", "2.0"),
        ("resultvar", "$alpha")
    ]);

    // Proxy handling using the `get_param` helper
    let proxies = vmt.proxies();
    println!("\nProxies found: {}", proxies.len());
    for (i, proxy) in proxies.iter().enumerate() {
        println!("Proxy {}: {}", i, proxy.name);
        if let Some(result_var) = proxy.get_param("resultVar") {
            println!("  Result Variable: {}", result_var);
        }
    }

    // Serialize back to a VMT-formatted string
    let result = vmt.to_string()?;

    println!("--- Updated VMT ---");
    println!("{}", result);

    Ok(())
}
