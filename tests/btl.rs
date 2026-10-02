use soulsformats_rs::{Btl, DcxIO};

#[test]
fn btl() {
    let (btl, compression) = Btl::decompress_file("./tests/files/btl/btl.dcx").unwrap();
    let compressed = btl.compress_to_bytes(compression).unwrap();
    let (round_trip, _) = Btl::decompress_bytes(compressed).unwrap();
    assert_eq!(btl.offsets_64bit, round_trip.offsets_64bit);
    assert_eq!(btl.version, round_trip.version);
    assert_eq!(btl.lights.len(), round_trip.lights.len());

    for index in 0..btl.lights.len() {
        assert_eq!(btl.lights[index], round_trip.lights[index]);
    }
}
