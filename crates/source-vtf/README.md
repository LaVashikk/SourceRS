# source-vtf

Pure Rust access to Valve Texture Format files used by Source Engine games.

Reads PC VTF 7.0 through 7.5. Parsing a byte slice is zero-copy; `Vtf::open` owns the file bytes but goes through the same parser, so thumbnails, resources, and subresources are always slices of the file. Parsed metadata is plain structs with public fields.

## Example

```rust,no_run
use source_vtf::{ResourceTag, Vtf};

fn main() -> Result<(), source_vtf::Error> {
    let vtf = Vtf::open("materials/example.vtf")?;
    let header = &vtf.header;
    println!("{}x{} {:?}, {} mips", header.width, header.height, header.image_format, header.mip_count);

    // Full-size mip of frame 0, face 0, depth slice 0
    let image = vtf.subresource(0, 0, 0, 0)?;
    println!("{} raw bytes", image.data.len());

    if let Some(sheet) = vtf.resource(ResourceTag::PARTICLE_SHEET_DATA) {
        println!("particle sheet, {} bytes", sheet.len());
    }
    Ok(())
}
```

`mip = 0` addresses the largest mip even though VTF stores the smallest mip first. Frame, cubemap face, and volume slice indices follow it in that order.

## Support

| Feature | Status |
| --- | --- |
| PC VTF 7.0-7.5 headers | Supported |
| Legacy 7.0-7.2 thumbnail and image layout | Supported |
| VTF 7.3+ resource dictionary, inline and sized resources | Supported |
| Mips, animation frames, cubemaps (with the 7.1-7.4 spheremap face), volume slices | Supported |
| Raw thumbnail and subresource views, byte sizes for format IDs 0-38 | Supported |
| Pre-7.5 NV_NULL/ATI2N/ATI1N IDs | Mapped to their 7.5 IDs |
| Pre-7.5 depth-buffer formats (IDs 30-35) | Rejected as unsupported |
| Pixel decoding and `image` adapters | Planned behind features |
| VTF writing | Planned after the reader API stabilizes |
| VTF 7.6 / Strata compression | Not part of this release |
| Xbox 1, Xbox 360, and PS3 variants | Not part of this release |

Block-compressed DXT1, DXT3, DXT5, ATI1N, and ATI2N payloads are addressed in whole 4x4 blocks.

## Features

- `serde`: serialization for the metadata types.

No feature is enabled by default.

## License
MIT License.
