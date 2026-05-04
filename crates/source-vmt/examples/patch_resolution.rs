#[cfg(feature = "material_system")]
use source_vmt::MaterialSystem;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "material_system")]
    {
        // Initialize MaterialSystem directly from a game path.
        // This uses source_fs::create_fs internally to parse gameinfo.txt.
        let mut mat_sys = source_vmt::MaterialSystem::<source_fs::providers::DummyVpk>::from_path("examples/pseudo_game/game")?
            .with_search_path("game")
            .prioritize_vpks(false);

        println!("Loading 'materials/wall_mossy.vmt'...");

        // Fetch the material. In the new system, get_material returns it as-is (unresolved).
        let raw_vmt = mat_sys.get_material("materials/wall_mossy.vmt")?;
        println!("Raw Shader: {}", raw_vmt.shader);

        // To get the patched/resolved version, use get_resolved_material.
        let vmt = mat_sys.get_resolved_material("materials/wall_mossy.vmt")?;

        println!("--- Final Resolved VMT ---");
        println!("Shader: {}", vmt.shader);
        println!("Base Texture: {:?}", vmt.get_string("basetexture"));
        println!("Detail: {:?}", vmt.get_string("detail"));
        println!("Phong Enabled: {}", vmt.get_bool("phong"));

        if let Some(path) = mat_sys.get_material_path("materials/wall_mossy.vmt") {
             println!("Material path: {:?}", path);
        }

        println!("\n--- Serialized Result ---");
        println!("{}", vmt.to_string()?);
    }

    #[cfg(not(feature = "material_system"))]
    println!("Please run this example with --features material_system");

    Ok(())
}
