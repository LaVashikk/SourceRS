use std::collections::HashMap;
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("examples/data/example.vmt");
    let content = fs::read_to_string(path)?;

    println!("Reading VMT from: {:?}", path);

    let vmt: HashMap<String, HashMap<String, Vec<String>>> = source_kv::from_str(&content)?;

    println!("\nDeserialized VMT as HashMap:");
    for (shader_name, properties) in &vmt {
        println!("Shader: {}", shader_name);
        for (key, values) in properties {
            println!("  {}: {:?}", key, values[0]);
        }
    }

    let serialized = source_kv::to_string(&vmt);
    println!("\nSerialized VMT:\n{}", serialized.unwrap());

    Ok(())
}
