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
    let content = fs.read_str("scripts/game_sounds.txt", "game")?;
    println!("Found file!\n{content}");
    Ok(())
}
```

Lookups walk one ordered list per search path ID, with loose dirs and VPKs interleaved as in `gameinfo.txt`. A directory's own `*_dir.vpk` archives sit above its loose files, and `dir/*` mounts every subfolder and VPK inside, alphabetically.

### Finding where a file comes from

```rust
use source_fs::{create_fs, FileLocation};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fs = create_fs("path/to/Portal 2/portal2")?;
    match fs.find("materials/metal/black_wall_metal_002a.vmt", "game") {
        Some(FileLocation::Loose(path)) => println!("loose file {}", path.display()),
        Some(FileLocation::Packed { entry, .. }) => println!("packed entry {entry}"),
        None => println!("not found"),
    }
    Ok(())
}
```

### Game-specific mounts

`gameinfo.txt` does not describe everything a game mounts. Pick a provider that adds the rest:

- `providers::SimpleGameInfo` - `gameinfo.txt` only.
- `providers::P2GameInfo` - Portal 2: `update/` and `portal2_dlcN/` above gameinfo paths.
- `providers::SimpleWithMount` - Garry's Mod / P2CE: games listed in `cfg/mount.cfg`.

The building blocks (`providers::portal2_dlc`, `providers::mount_cfg`) are public, so combining them in your own `GameInfoProvider` takes a few lines.

```rust
use source_fs::providers::P2GameInfo;
use source_fs::{FileSystem, FileSystemOptions};

let options = FileSystemOptions {
    // Sourcemods resolve `hl2`, `platform` etc. against the SDK Base install
    base_dir: Some("path/to/Source SDK Base 2013 Singleplayer".into()),
    ..Default::default()
};
let fs: FileSystem = FileSystem::load_from_path::<P2GameInfo>("path/to/portal2".as_ref(), &options)?;
```

### Using Steam auto-discovery (requires `steam` feature)

```rust
#[cfg(feature = "steam")]
use source_fs::providers::P2GameInfo;
#[cfg(feature = "steam")]
use source_fs::{FileSystem, FileSystemOptions};

#[cfg(feature = "steam")]
fn main() {
    // Locates the Steam installation and mounts Portal 2 (AppID 620)
    let fs: FileSystem = FileSystem::load_from_app_id::<P2GameInfo>(620, "portal2", &FileSystemOptions::default())
        .expect("Failed to locate game via Steam");

    let file = fs
        .read_str("scripts/vscripts/mapspawn.nut", "game")
        .expect("Failed to read file");
    println!("Found file:\n{}", file);
}
```

## Features

- `vpk` (default) - mounts VPK archives through `source-vpk`. Without it `DefaultPack` is `DummyVpk` and only loose files are visible.
- `steam` - `FileSystem::load_from_app_id`.

## API

- `source_fs::create_fs` - Load a game dir with `SimpleGameInfo` and the default pack backend.
- `source_fs::FileSystem` - Search path IDs mapped to ordered mounts.
  - `.load_from_path::<G>()` / `.load_from_app_id::<G>()` - Load with a `GameInfoProvider`.
  - `.find()` / `.find_asset()` - Where a file comes from (`FileLocation::Loose` / `FileLocation::Packed`).
  - `.open()` / `.read()` / `.read_str()` - Stream or read a file.
  - `.mount_dir()` / `.mount_vpk()` - Build a filesystem by hand.
- `source_fs::PackFile` - Abstract trait to plug in your own archive backend.
- `source_fs::providers::GameInfoProvider` - Turns a game install into search paths.

## License
MIT License.
