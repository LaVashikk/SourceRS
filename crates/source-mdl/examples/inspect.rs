use source_mdl::{GeometryOptions, StudioModel};

fn main() -> Result<(), source_mdl::Error> {
    let path = std::env::args_os()
        .nth(1)
        .unwrap_or_else(|| "model.mdl".into());
    let model = StudioModel::open(path)?;
    let geometry = model.geometry(&GeometryOptions::default())?;
    let mdl = model.mdl();

    println!(
        "{}: MDL {}, {} bones, {} materials, {} vertices, {} meshes",
        mdl.header.name,
        mdl.header.version,
        mdl.bones.len(),
        mdl.materials.len(),
        geometry.vertices.len(),
        geometry.meshes.len(),
    );
    Ok(())
}
