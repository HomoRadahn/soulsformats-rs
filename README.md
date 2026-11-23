# soulsformats-rs
Rust library for reading and writing formats used by FromSoftware games. Based on SoulsFormatsNEXT .NET Library.


## Status
The library is under active development. Most formats aren't yet implemented. Don't expect a flawless experience.

### Currently supports:
- Binder family of formats
- Dcx compression
- Btl (light sources in map)
- Fmg (text data)
- Param files with XML or binary ParamDef definitions

## Example
For an example use, see `examples/decompress-bnd4.rs`. You can run it with:
```
cargo run --example decompress-bnd4
```

For integration tests, which can serve as crude examples, see `tests` folder.

To edit a PARAM, load its matching XML or binary PARAMDEF, apply it, then edit
the decoded cells before writing:

```rust,no_run
use soulsformats_rs::{ByteIO, FileIO, Param, ParamDef};

fn main() -> std::io::Result<()> {
    let definition = ParamDef::from_xml("definition.xml", false)?;
    let mut param = Param::from_file("data.param")?;
    param.apply_paramdef(&definition)?;
    if let Some(row) = param.find_row_mut(100) {
        if let Some(cell) = row.get_cell_mut("someField") {
            cell.set(42_i32)?;
        }
    }
    let bytes = param.to_bytes()?;
    Ok(())
}
```

For version-aware definitions, use `apply_regulation_versioned_paramdef` when
reading or `Param::new_for_version` when creating a PARAM. Rows and cells share
their definitions through `Arc`, so cloning them does not duplicate field
metadata for every cell.

Generate API documentation, including examples, with `cargo doc --open`.

## Credits
- [SoulsFormatsNEXT](https://github.com/soulsmods/SoulsFormatsNEXT) (and the original SoulsFormats library). Without the authors' work, this project would never exist.
- [oodle-rs](https://github.com/meszmate/oodle-rs)
- [flate2](https://docs.rs/flate2/latest/flate2/)
- [zstd](https://docs.rs/zstd/latest/zstd/)
- [encoding_rs](https://docs.rs/encoding_rs/latest/encoding_rs/)
- [bitflags](https://docs.rs/bitflags/latest/bitflags/)
- [libloading](https://docs.rs/libloading/latest/libloading/)
- [chrono](https://docs.rs/chrono/latest/chrono/)

## Note
This will only ever support formats up to Armored Core VI / Elden Ring, unless a few other people will be willing to help out with this project. Right now this project is managed and maintained by only one person and I don't have the time, will or need to support later releases, like the upcoming Duskbloods.

## TODO for next pre-release
- [x] Optimizations in format reading / writing
- [x] Param, Row, Cell
- [ ] MSB formats
- [x] Improve documentation and add doc tests