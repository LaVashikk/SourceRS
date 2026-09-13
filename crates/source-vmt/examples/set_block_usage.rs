use source_vmt::Vmt;
use serde::{Serialize, Deserialize};

// Define custom blocks for material properties
#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct PbrSettings {
    #[serde(rename = "$metalness")]
    metalness: f32,
    #[serde(rename = "$roughness")]
    roughness: f32,
    #[serde(rename = "$ao")]
    ambient_occlusion: f32,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct ReplaceSettings {
    #[serde(rename = "$phongboost")]
    boost: f32,
    #[serde(rename = "$phongexponent")]
    exponent: i32,
    #[serde(rename = "$phongfresnelranges")]
    fresnel_ranges: String,
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== set_block Example: Creating Materials with Structured Blocks ===\n");

    // Example 1: Create a PBR material
    println!("--- Example 1: Creating PBR Material ---");
    let mut pbr_material = Vmt::new("PBR");
    pbr_material.set_string("$basetexture", "materials/metal/steel_plate");

    let pbr_settings = PbrSettings {
        metalness: 0.9,
        roughness: 0.3,
        ambient_occlusion: 1.0,
    };

    pbr_material.set_block("pbr_settings", &pbr_settings)?;

    println!("Created PBR material:");
    println!("{}", pbr_material.to_string()?);

    // Verify we can read it back
    let retrieved_pbr: PbrSettings = pbr_material.get_block("pbr_settings")?.unwrap();
    println!("Retrieved PBR settings: metalness={}, roughness={}, ao={}\n",
        retrieved_pbr.metalness, retrieved_pbr.roughness, retrieved_pbr.ambient_occlusion);

    // Example 2: Create a base material and a patch that extends it
    println!("--- Example 2: Patch with set_block ---");

    let mut patch_material = Vmt::new("Patch");
    patch_material.set_string("include", "materials/base_material.vmt");

    // Override phong settings in the patch
    let patch_phong = ReplaceSettings {
        boost: 3.0,
        exponent: 50,
        fresnel_ranges: "[1.0 2.0 3.0]".to_string(),
    };
    patch_material.set_block("replace", &patch_phong)?;

    println!("Patch material:");
    println!("{}", patch_material.to_string()?);

    Ok(())
}
