use source_vpk::Vpk;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: cargo run --example list -- <path-to-dir.vpk>")?;
    let vpk = Vpk::open(path)?;

    println!("{} entries, VPK v{}", vpk.len(), vpk.version().number());
    for (path, entry) in vpk.entries() {
        println!("{path}\t{} bytes\t{:?}", entry.len(), entry.archive());
    }

    Ok(())
}
