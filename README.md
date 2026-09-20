# soulsformats-rs
Rust library for reading and writing formats used by FromSoftware games. Based on SoulsFormatsNEXT .NET Library.


## Status
The library is under active development. Most formats aren't yet implemented. Don't expect a flawless experience.

### Currently supports:
- Binder family of formats (`.*bnd`, `.bdt`, `.bhd`)
- DCX compression
- BTL (light sources in map)

## Example
For an example use, see `examples/decompress-bnd4.rs`. You can run it with:
```
cargo run --example decompress-bnd4
```

## Credits
- [SoulsFormatsNEXT](https://github.com/soulsmods/SoulsFormatsNEXT) by the `soulsmods` group on GitHub (and JKAnderson for the original SoulsFormats library). Without their work, this library would never exist.
- [oodle-rs](https://github.com/meszmate/oodle-rs) by `meszmate`
- [flate2](https://docs.rs/flate2/latest/flate2/)
- [zstd](https://docs.rs/zstd/latest/zstd/)
- [encoding_rs](https://docs.rs/encoding_rs/latest/encoding_rs/)
- [bitflags](https://docs.rs/bitflags/latest/bitflags/)
- [libloading](https://docs.rs/libloading/latest/libloading/)