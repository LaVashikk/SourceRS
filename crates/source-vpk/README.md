# source-vpk

Pure Rust support for Valve VPK archives used by Source Engine games.

The current implementation reads VPK v1 and v2 directory files, including preload data, files embedded in the directory VPK, and files stored in numbered external archives. Entries are exposed through a bounded `Read + Seek` reader, so large files do not have to be copied into memory.

## Example

```rust,no_run
use source_vpk::Vpk;
use std::io::Read;

fn main() -> Result<(), source_vpk::Error> {
    let vpk = Vpk::open("path/to/pak01_dir.vpk")?;
    let mut file = vpk.open_entry("materials/example.vmt")?;

    let mut text = String::new();
    file.read_to_string(&mut text)?;
    println!("{text}");

    Ok(())
}
```

## Support

| Feature | Status |
| --- | --- |
| VPK v1 directory and entry reading | Supported |
| VPK v2 directory and entry reading | Supported |
| Preload data | Supported |
| Directory-embedded data | Supported |
| Numbered external archives | Supported |
| Entry CRC32 verification | Supported |
| VPK v2 directory footer MD5 verification | Supported |
| VPK v2 external archive chunk verification | Planned |
| VPK v1/v2 writing | Planned |
| VPK v54 compression | Not Planned |

## License
MIT License.