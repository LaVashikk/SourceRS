# source-vtf

Pure Rust access to Valve Texture Format files used by Source Engine games.

The current reader covers PC VTF 7.0 through 7.5. Parsing from a byte slice is zero-copy; opening a path owns the file bytes, but uses the same parser and the same borrowed `RawImage` views.

## Example

```rust,no_run
use source_vtf::Vtf;

fn main() -> Result<(), source_vtf::Error> {
    let vtf = Vtf::open("materials/example.vtf")?;
    let image = vtf.subresource(0, 0, 0, 0)?;

    println!(
        "{}x{} {:?}: {} raw bytes",
        image.width(),
        image.height(),
        image.format(),
        image.len(),
    );
    Ok(())
}
```

`mip = 0` addresses the largest mip even though VTF stores the smallest mip first. Frame, cubemap face, and volume slice indices follow it in that order.

## Current support

| Feature | Status |
| --- | --- |
| PC VTF 7.0–7.5 headers | Supported |
| Legacy 7.0–7.2 thumbnail/image layout | Supported |
| VTF 7.3+ resource dictionary | Supported |
| Inline and offset-backed resource inspection | Supported |
| Mips, animation frames, cubemaps, and volume slices | Supported |
| Borrowed raw thumbnail and subresource views | Supported |
| Checked sizing for VTF 7.5 image-format IDs 0–38 | Supported |
| Pre-7.5 shifted NV_NULL/ATI2N/ATI1N IDs | Supported and normalized |
| Pre-7.5 depth-buffer-only IDs 33–35 | Rejected as unsupported image payloads |
| Pixel decoding and `image` adapters | Planned behind features |
| VTF writing | Planned after the reader API stabilizes |
| VTF 7.6 / Strata compression | Not part of this release |
| Xbox 1, Xbox 360, and PS3 variants | Not part of this release |

Raw access does not depend on a decoder. Block-compressed DXT1, DXT3, DXT5, ATI1N, and ATI2N payloads are addressed with complete 4×4 block rounding.

## Features

- `serde`: serialization support for public metadata types.

No feature is enabled by default.

## Layout

The crate keeps wire-format concerns separate without introducing a generic binary parsing framework:

- `header`: common and version-gated header fields;
- `format`: checked format IDs and byte-size rules;
- `resource`: legacy data locations and the 7.3+ resource dictionary;
- `subresource`: physical mip/frame/face/slice layout;
- `vtf`: owned/borrowed facade and raw views;
- `cursor`, `flags`, and `error`: small supporting modules with crate-local scope.

## License
MIT License.