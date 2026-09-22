use soulsformats_rs::{BTL, ByteIO, DCX};

#[test]
fn btl() {
    let dcx = DCX::from_file("./tests/files/btl/btl.dcx").unwrap();
    let btl = BTL::from_bytes(dcx.data).unwrap();
    let compressed = btl.to_bytes().unwrap();
    let round_trip = BTL::from_bytes(compressed).unwrap();
    assert_eq!(btl.offsets_64bit, round_trip.offsets_64bit);
    assert_eq!(btl.version, round_trip.version);
    assert_eq!(btl.lights.len(), round_trip.lights.len());

    for index in 0..btl.lights.len() {
        assert_eq!(btl.lights[index], round_trip.lights[index]);
    }
}
