# source-mdl

Pure Rust access to Source Engine studio models: MDL, VVD, and VTX.

The core reads MDL 44-49, VVD 4, and VTX 7 from byte slices or paths and does not depend on game search paths or material parsers. `Mdl` loads on its own for metadata-only use; `StudioModel` validates the three files against each other and extracts geometry for a selected LOD, skin, and bodygroup set. Parsed data is plain structs with public fields.

## Example

```rust,no_run
use source_mdl::{GeometryOptions, StudioModel};

fn main() -> Result<(), source_mdl::Error> {
    // Finds prop.vvd and prop.dx90.vtx (or .vtx, .dx80.vtx, .sw.vtx) next to it.
    let model = StudioModel::open("prop.mdl")?;
    let geometry = model.geometry(&GeometryOptions { lod: 0, skin: 0, ..Default::default() })?;

    for mesh in &geometry.meshes {
        println!(
            "{} triangles, material {:?}",
            mesh.indices.len() / 3,
            mesh.material.path_candidates,
        );
    }
    Ok(())
}
```

With the `model_system` feature, `ModelSystem` loads models through `source-fs`, caches them, and resolves their materials through the `source_vmt::MaterialSystem` it owns:

```rust,ignore
use source_mdl::{GeometryOptions, ModelSystem};

let mut models = ModelSystem::from_path("path/to/portal2")?;
let model = models.get_model("props/metal_box")?;
let geometry = model.geometry(&GeometryOptions::default())?;
for mesh in &geometry.meshes {
    let vmt = models.get_material(&mesh.material)?;
}
```

## Support

| Feature | Status |
| --- | --- |
| MDL 44-49 header, bones, bone controllers, hitbox sets | Supported |
| Materials, material directories, skin families | Supported |
| Body parts, models, meshes, per-LOD vertex counts | Supported |
| Sequences and animation tracks stored in the MDL | Experimental |
| Animation data in external `.ani` blocks or sections | Detected, not decoded |
| VVD 4 vertices, tangents, and LOD fixups | Supported |
| VTX 7 strip groups, strips, and LOD material replacements | Supported |
| Triangle-list extraction for LOD, skin, and bodygroup selections | Supported |
| Rest pose, single-frame sequence pose, CPU skinning | Experimental |
| Sequence blending, flexes, attachments, IK, eyeballs, PHY | Not part of this release |
| Console variants, MDL versions after 49, Source 2 models | Not part of this release |
| Writing | Not part of this release |

Animation tracks that fail to parse do not fail the model: they are left empty and `Animation::unsupported` says why.

## License
MIT License.
