use soulsformats_rs::{ByteIO, FileIO, bnd::BND4};

#[test]
fn bnd4_read() {
    let bnd = BND4::from_file("./tests/files/bnd/test.bnd.dcx").unwrap();
    let text1 = String::from_utf8(bnd.files[0].bytes.clone()).unwrap();
    let text2 = String::from_utf8(bnd.files[1].bytes.clone()).unwrap();
    assert_eq!(text1, "BND4 Test");
    assert_eq!(text2, "BND4 Test 2");
}

#[test]
fn bnd4_write_round_trip() {
    let bnd = BND4::from_file("./tests/files/bnd/test.bnd.dcx").unwrap();
    let round_trip = BND4::from_bytes(bnd.to_bytes().unwrap()).unwrap();

    assert_eq!(bnd.version, round_trip.version);
    assert_eq!(bnd.format, round_trip.format);
    assert_eq!(bnd.unk04, round_trip.unk04);
    assert_eq!(bnd.unk05, round_trip.unk05);
    assert_eq!(bnd.unicode, round_trip.unicode);
    assert_eq!(bnd.extended, round_trip.extended);
    assert_eq!(bnd.files.len(), round_trip.files.len());
    assert_eq!(bnd.compression, round_trip.compression);

    for (file, round_trip_file) in bnd.files.iter().zip(round_trip.files.iter()) {
        assert_eq!(file.flags, round_trip_file.flags);
        assert_eq!(file.id, round_trip_file.id);
        assert_eq!(file.name, round_trip_file.name);
        assert_eq!(file.bytes, round_trip_file.bytes);
        assert_eq!(file.compression, round_trip_file.compression);
    }
}
