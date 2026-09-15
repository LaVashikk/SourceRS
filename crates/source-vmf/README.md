# source-vmf

A straightforward, high-performance library for parsing, manipulating, and serializing Valve Map Format (VMF) files used in Source Engine games.

It provides a strongly-typed representation of map geometry, entities, visgroups, cameras, and cordons, while preserving unknown keys and custom blocks for lossless round-tripping.

## Key Features

* **Lossless Round-tripping**: Preserves unknown properties and custom vendor blocks across parse and serialize cycles.
* **Complete Map Coverage**: Strongly-typed structures for version info, visgroups, view settings, world geometry (solids, sides), point and brush entities, cameras, and cordons.
* **Fast Entity Lookups**: Search entities across visible and hidden blocks by targetname or name prefix using `EntityIndex`.
* **Entity I/O Connections**: Ergonomic API for reading, modifying, and creating entity logic connections.
* **Atomic Saving**: Writes safely to a temporary `.part` file before replacing the target file on disk.
* **Map Merging**: Built-in helper to merge geometry, entities, visgroups, and cordons from multiple VMF files.
* **Serde Support**: Optional `serialization` feature for serializing and deserializing VMF structures.

### Feature Flags

* `serialization` — Enables Serde `Serialize` and `Deserialize` implementations for VMF data structures.

## Quick Start

### 1. Reading, Modifying, and Saving a Map

```rust,no_run
use source_vmf::prelude::*;

fn main() -> Result<(), VmfError> {
    let mut vmf = VmfFile::open("maps/de_dust2.vmf")?;

    println!("Map version: {}", vmf.versioninfo.map_version);

    // Find entities by classname
    for player_start in vmf.entities.find_by_classname("info_player_start") {
        println!("Spawn origin: {:?}", player_start.get("origin"));
    }

    // Create and add a new entity
    let mut tree = Entity::new("prop_static", 1000);
    tree.set("model".to_string(), "models/props_foliage/tree01.mdl".to_string());
    tree.set("origin".to_string(), "0 0 0".to_string());
    vmf.entities.push(tree);

    // Save atomically to disk
    vmf.save("maps/de_dust2_modified.vmf")?;

    Ok(())
}
```

### 2. Fast Lookups with EntityIndex

```rust,no_run
use source_vmf::prelude::*;

fn main() -> Result<(), VmfError> {
    let vmf = VmfFile::open("maps/test.vmf")?;
    let index = EntityIndex::build(&vmf);

    // Exact match by targetname (includes hidden entities)
    for &id in index.by_name("door_1") {
        if let Some(ent) = vmf.entity(id) {
            println!("Found door: {:?}", ent.classname());
        }
    }

    // Prefix search (e.g., all entities starting with "door_")
    for id in index.by_prefix("door_") {
        if let Some(ent) = vmf.entity(id) {
            println!("Matched entity: {:?}", ent.targetname());
        }
    }

    Ok(())
}
```

### 3. Adding Logic Connections

```rust
use source_vmf::prelude::*;

let mut button = Entity::new("func_button", 42);
button.add_connection(
    "OnPressed",
    "door_main",
    "Open",
    "",
    0.0,
    -1,
);
```

### 4. Merging Maps

```rust,no_run
use source_vmf::prelude::*;

fn main() -> Result<(), VmfError> {
    let mut base_map = VmfFile::open("maps/base.vmf")?;
    let prefab = VmfFile::open("maps/prefab.vmf")?;

    // Merges world solids, entities, hidden entities, visgroups, and cordons
    base_map.merge(prefab);
    base_map.save("maps/combined.vmf")?;

    Ok(())
}
```

## License
MIT License.