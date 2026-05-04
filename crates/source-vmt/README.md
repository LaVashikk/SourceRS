# source-vmt

A high-performance, ergonomic Rust library for parsing, manipulating, and resolving Source Engine VMT (Valve Material Type) files. 

Beyond basic parsing, `source-vmt` provides a complete material management ecosystem, including VFS integration, recursive patch resolution, and material proxy support.

## Key Features

* **Ergonomic API**: Automatic handling of `$` and `%` prefixes, with type-safe getters for Source-specific formats (e.g., parsing `[0.0 0.5 1.0]` or `{255 128 0}` into normalized colors).
* **Patch Resolution**: Deep inheritance support for the `patch` shader, automatically merging `insert` and `replace` blocks.
* **Material System**: An optional high-level manager integrating with `source-fs` for caching, file path tracking, fallback materials, and procedural material registration.
* **Serde Integration**: Extract and deserialize custom VMT blocks directly into strongly-typed Rust structs.
* **Material Proxies**: Dedicated API for extracting, modifying, and adding material proxies.
* **Memory Optimization**: Optional string interning (`intern_keys` feature) via global pooling to drastically reduce memory footprint when loading thousands of materials.

### Feature Flags

* `material_system` — Enables the `MaterialSystem` and pulls in `source-fs` for virtual filesystem integration.
* `intern_keys` — Enables key deduplication using `dashmap` wtih `Arc` to optimize memory usage.

## Quick Start

### 1. Basic Editing & Fluent API

```rust
use source_vmt::Vmt;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = r#"
        "VertexLitGeneric"
        {
            "$basetexture" "models/player/scout/scout_red"
            "$surfaceprop" "flesh"
        }
    "#;

    let mut vmt = Vmt::from_str(input)?;

    // Chain property mutations easily
    vmt.set_flag("$rimlight", true)
       .set_string("$rimlightexponent", "5")
       .remove("$surfaceprop");

    println!("{}", vmt.to_string()?);
    Ok(())
}
```

### 2. Type-Safe Access & Serde Blocks

The library automatically handles key normalization (case-insensitivity, prefixes).

```rust
// Automatically parses Source color vectors/bytes into [f32; 3]
if let Some(color) = vmt.get_color("$envmaptint") {
    println!("Normalized color: {:?}", color);
}

// Extract a nested block into a custom struct using Serde
#[derive(serde::Deserialize)]
struct PbrParams {
    #[serde(rename = "$bumpmap")]
    bumpmap: String,
    #[serde(rename = "$mraotexture")]
    mrao: String,
}

if let Some(pbr) = vmt.get_block::<PbrParams>("PBR")? {
    println!("MRAO Texture: {}", pbr.mrao);
}
```

### 3. Managing Material Proxies

You can easily read and construct `Proxies` blocks.

```rust
// Add a new proxy
vmt.add_proxy("Sine", [
    ("sineperiod", "2.0"),
    ("resultVar", "$alpha")
]);

// Iterate over existing proxies
for proxy in vmt.proxies() {
    println!("Proxy: {}", proxy.name);
    if let Some(result_var) = proxy.get_param("resultVar") {
        println!("Target: {}", result_var);
    }
}
```

### 4. Patch Resolution & Material System (Requires `material_system`)

Use the `MaterialSystem` to automatically locate materials via `gameinfo.txt` search paths and resolve recursive `patch` shaders.

```rust
use source_vmt::MaterialSystem;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initializes the VFS and material cache
    let mut mat_sys = MaterialSystem::from_path("path/to/game_dir")?;
    
    // get_resolved_material automatically finds the file, parses it, 
    // and applies all 'include' patches recursively.
    let vmt = mat_sys.get_resolved_material("materials/wall_mossy.vmt")?;
    
    println!("Final resolved shader: {}", vmt.shader);
    println!("Base Texture: {:?}", vmt.get_string("basetexture"));
    
    Ok(())
}
```

## License
MIT License.