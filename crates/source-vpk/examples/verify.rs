use source_vpk::Vpk;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: cargo run --example verify -- <path-to-dir.vpk>")?;
    let vpk = Vpk::open(path)?;
    let mut failed = 0;

    for (path, _) in vpk.entries() {
        if let Err(error) = vpk.verify_entry(path) {
            failed += 1;
            eprintln!("{path}: {error}");
        }
    }

    if let Some(checksums) = vpk.verify_archive()? {
        println!("directory checksums: {checksums:?}");
        if !checksums.is_valid() {
            failed += 1;
        }
    }

    if failed == 0 {
        println!("verified {} entries", vpk.len());
        Ok(())
    } else {
        Err(format!("{failed} checksum checks failed").into())
    }
}
