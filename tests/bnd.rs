use soulsformats_rs::{
    ByteIO, FileIO,
    binder::{BND3, BND4},
};

#[test]
fn bnd3_round_trip() {
    println!("Note this BND3 is Kraken compressed, Oodle required");
    let bnd = BND3::from_file("./tests/files/bnd/bnd3.dcx").unwrap();
    assert_eq!(
        bnd.files[0].bytes,
        vec![
            0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA,
            0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA,
            0xAA, 0xAA, 0xAA, 0xAA
        ]
    );
    assert_eq!(
        bnd.files[1].bytes,
        vec![
            0xBB, 0xBB, 0xBB, 0xBB, 0xBB, 0xBB, 0xBB, 0xBB, 0xBB, 0xBB, 0xBB, 0xBB, 0xBB, 0xBB,
            0xBB, 0xBB, 0xBB, 0xBB, 0xBB, 0xBB, 0xBB, 0xBB, 0xBB, 0xBB, 0xBB, 0xBB, 0xBB, 0xBB,
            0xBB, 0xBB, 0xBB, 0xBB
        ]
    );
    let round_trip = BND3::from_bytes(bnd.to_bytes().unwrap()).unwrap();

    assert_eq!(bnd.version, round_trip.version);
    assert_eq!(bnd.format, round_trip.format);
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

#[test]
fn bnd4_round_trip() {
    let bnd = BND4::from_file("./tests/files/bnd/bnd4.dcx").unwrap();
    let text1 = String::from_utf8(bnd.files[0].bytes.clone()).unwrap();
    let text2 = String::from_utf8(bnd.files[1].bytes.clone()).unwrap();
    assert_eq!(text1, "BND4 Test");
    assert_eq!(text2, "BND4 Test 2");
    let round_trip = BND4::from_bytes(bnd.to_bytes().unwrap()).unwrap();

    assert_eq!(bnd.version, round_trip.version);
    assert_eq!(bnd.format, round_trip.format);
    assert_eq!(bnd.unk_04, round_trip.unk_04);
    assert_eq!(bnd.unk_05, round_trip.unk_05);
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
