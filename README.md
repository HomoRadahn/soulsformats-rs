# soulsformats-rs
Rust library for reading and writing formats used by FromSoftware games. Based on SoulsFormatsNEXT .NET Library.


## Status
The library is under active development. Most formats aren't yet implemented. Don't expect a flawless experience.

### Currently supports:
- Binder family of formats
- Dcx compression
- Btl (light sources in map)
- Fmg (text data)

## Example
For an example use, see `examples/decompress-bnd4.rs`. You can run it with:
```
cargo run --example decompress-bnd4
```

For integration tests, which can serve as crude examples, see `tests` folder.

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
