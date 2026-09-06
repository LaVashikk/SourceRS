# source-fs

A straightforward library for building and querying Source Engine virtual filesystems. 

It automatically parses `gameinfo.txt`, correctly resolves Valve's `SearchPaths` priorities, handles macro expansions (like `|all_source_engine_paths|`), and provides case-insensitive file resolution for Unix-like systems.

Inspired by [craftablescience/sourcepp](https://github.com/craftablescience/sourcepp).

## Quick Start

### Loading the FileSystem and reading a file

```rust
use source_fs::create_fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fs = create_fs("path/to/Half-Life 2/hl2")?;

    match fs.read_str("scripts/game_sounds.txt", "game", false)? {
        Some(content) => println!("Found file!\n{content}"),
        None => println!("File not found in the virtual filesystem."),
    }
    Ok(())
}
```

### Loading VPK search paths

`create_fs` keeps the loose-file-only behavior. Use `create_vpk_fs` when `gameinfo.txt` contains explicit `.vpk` search paths:

```rust
use source_fs::create_vpk_fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fs = create_vpk_fs("path/to/game_dir")?;
    let bytes = fs.read("materials/example.vmt", "game", false)?
        .ok_or("material not found")?;

    println!("read {} bytes", bytes.len());
    Ok(())
}
```

`Vpk` is also re-exported for direct use with the generic API: `FileSystem::<source_fs::Vpk>::load_from_path(...)`.

### Using Steam auto-discovery (requires `steam` feature)

```toml
[dependencies]
source-fs = { version = "0.1.0", features = ["steam"] }
```

```rust
#[cfg(feature = "steam")]
use source_fs::{FileSystem, FileSystemOptions, SimpleGameInfo, providers::DummyVpk};

#[cfg(feature = "steam")]
fn main() {
    let options = FileSystemOptions::default();
    
    // Automatically locates the Steam installation and mounts Portal 2 (AppID 620)
    let fs = FileSystem::<DummyVpk>::load_from_app_id::<SimpleGameInfo>(620, "portal2", &options)
        .expect("Failed to locate game via Steam");

    let file = fs
        .read_str("scripts/vscripts/mapspawn.nut", "game", false)
        .expect("Failed to read file")
        .expect("File not found");
    println!("Found file:\n{}", file);
}
```

## API

- `source_fs::create_fs` - Initialize a loose-file-only filesystem.
- `source_fs::create_vpk_fs` - Initialize a filesystem with VPK search paths enabled.
- `source_fs::VpkFileSystem` - Convenience alias for `FileSystem<source_fs::Vpk>`.
- `source_fs::FileSystem` - The core struct managing search paths and mounted archives.
  - `.load_from_path::<G>()` - Load custom `GameInfoProvider`.
  - `.read()` - Read a file as raw bytes (`Vec<u8>`).
  - `.read_str()` - Read a file as a UTF-8 String.
  - `find_file()` - Find a file in the virtual filesystem.
- `source_fs::traits::PackFile` - Abstract trait to implement your own VPK parser.
- `source_fs::traits::GameInfoProvider` - Abstract trait to parse custom game configurations (e.g. Portal 2 DLC system).

## License
MIT License.
