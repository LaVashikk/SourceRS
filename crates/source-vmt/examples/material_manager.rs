#[cfg(feature = "material_system")]
use source_vmt::{Vmt, MaterialSystem};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "material_system")]
    {
        // Initialize MaterialSystem.
        // It now requires a FileSystem. We can create a basic one from a path.
        // For this example, we'll use a dummy/empty path or the example game path.
        let mut mat_sys = MaterialSystem::<source_fs::providers::DummyVpk>::from_path("examples/pseudo_game/game")?
            .with_search_path("game")
            .prioritize_vpks(true);

        // Setup a Fallback material
        let mut error_vmt = Vmt::new("UnlitGeneric");
        error_vmt.set_string("$basetexture", "debug/debugempty");
        mat_sys.set_fallback(error_vmt);

        // Register a procedural material
        let mut shield_vmt = Vmt::new("VertexLitGeneric");
        shield_vmt.set_string("$basetexture", "effects/shield_diffuse")
                  .set_flag("$translucent", true)
                  .set_string("$color", "[0 0.5 1]");
        mat_sys.register("proc/player_shield", shield_vmt);

        // Fetch missing -> fallback
        println!("Fetching missing material...");
        let m1 = mat_sys.get_material("materials/not_found.vmt")?;
        println!("Shader: {} (Fallback)", m1.shader);
        println!("Texture: {:?}\n", m1.get_string("basetexture"));

        // Fetch procedural
        println!("Fetching procedural material...");
        let m2 = mat_sys.get_material("proc/player_shield")?;
        println!("Shield Color: {:?}\n", m2.get_string("color"));

        // Mutate material
        println!("Creating a unique Red Shield instance...");
        let mut red_shield = mat_sys.get_material_mut("proc/player_shield")?;
        red_shield.set_string("$color", "[1 0 0]");

        println!("New Shield Color: {:?}", red_shield.get_string("color"));
        println!("Original cached shield is still: {:?}", m2.get_string("color"));
    }

    #[cfg(not(feature = "material_system"))]
    println!("Please run this example with --features material_system");

    Ok(())
}
