#[cfg(feature = "patch_resolution")]
use source_vmt::MaterialSystem;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "patch_resolution")]
    {
        // 1. Initialize MaterialSystem directly from a game path.
        // This uses source_fs::create_fs internally to parse gameinfo.txt.
        let mut mat_sys = source_vmt::MaterialSystem::<source_fs::providers::DummyVpk>::from_path("examples/pseudo_game/game")?
            .with_search_path("game")
            .prioritize_vpks(false);

        println!("Loading 'materials/wall_mossy.vmt'...");

        // 2. Fetch the material. Patch resolution is automatic.
        let vmt = mat_sys.get_material("materials/wall_mossy.vmt")?;

        println!("--- Final Resolved VMT ---");
        println!("Shader: {}", vmt.shader);
        println!("Base Texture: {:?}", vmt.get_string("basetexture"));
        println!("Detail: {:?}", vmt.get_string("detail"));
        println!("Phong Enabled: {}", vmt.get_bool("phong"));

        println!("\n--- Serialized Result ---");
        println!("{}", vmt.to_string()?);
    }

    #[cfg(not(feature = "patch_resolution"))]
    println!("Please run this example with --features patch_resolution");

    Ok(())
}
