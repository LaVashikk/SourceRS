use source_vtf::Vtf;

fn main() -> Result<(), source_vtf::Error> {
    let path = std::env::args_os()
        .nth(1)
        .unwrap_or_else(|| "example.vtf".into());
    let vtf = Vtf::open(path)?;
    let header = &vtf.header;

    println!(
        "VTF 7.{}: {}x{}x{}, {:?}, {} mip(s), {} frame(s), {} face(s)",
        header.version.minor(),
        header.width,
        header.height,
        header.depth,
        header.image_format,
        header.mip_count,
        header.frame_count,
        header.face_count(),
    );
    for resource in &vtf.resources {
        println!("resource 0x{:06x} {:?}", resource.tag.0, resource.location);
    }

    Ok(())
}
