use soulsformats_rs::{
    ByteIO, DCX, FileIO,
    binder::{BND, BND2, BND3, BND4, bnd, bnd2},
};

#[test]
fn bnd() {
    let mut bnd = BND::empty();
    bnd.internal_version = 1;
    bnd.root_file_path = Some(String::from("L:\\"));
    bnd.files = vec![
        bnd::File::new(1, "こんにちは", vec![1, 2, 3, 4, 5, 6, 7]),
        bnd::File::new(2, "こんにちは", vec![1, 2, 3, 4, 5, 6, 7]),
    ];
    let round_trip = BND::from_bytes(bnd.to_bytes().unwrap()).unwrap();
    assert_eq!(bnd.internal_version, round_trip.internal_version);
    assert_eq!(bnd.format0, round_trip.format0);
    assert_eq!(bnd.format1, round_trip.format1);
    assert_eq!(bnd.root_file_path, round_trip.root_file_path);
    for index in 0..bnd.files.len() {
        assert_eq!(bnd.files[index].id, round_trip.files[index].id);
        assert_eq!(bnd.files[index].name, round_trip.files[index].name);
        assert_eq!(bnd.files[index].bytes, round_trip.files[index].bytes);
    }

    assert_eq!(bnd.to_bytes().unwrap(), round_trip.to_bytes().unwrap());
}

#[test]
fn bnd2_read() {
    let bnd = BND2::from_file("./tests/files/bnd/bnd2.bin").unwrap();
    let round_trip = BND2::from_bytes(bnd.to_bytes().unwrap()).unwrap();
    assert_eq!(bnd.header_info_flags, round_trip.header_info_flags);
    assert_eq!(bnd.file_info_flags, round_trip.file_info_flags);
    assert_eq!(bnd.unk_06, round_trip.unk_06);
    assert_eq!(bnd.unk_07, round_trip.unk_07);
    assert_eq!(bnd.file_version, round_trip.file_version);
    assert_eq!(bnd.alignment_size, round_trip.alignment_size);
    assert_eq!(bnd.unk_1b, round_trip.unk_1b);
    assert_eq!(bnd.base_directory, round_trip.base_directory);
    for index in 0..bnd.files.len() {
        assert_eq!(bnd.files[index].id, round_trip.files[index].id);
        assert_eq!(bnd.files[index].name, round_trip.files[index].name);
        assert_eq!(bnd.files[index].bytes, round_trip.files[index].bytes);
    }

    assert_eq!(bnd.to_bytes().unwrap(), round_trip.to_bytes().unwrap());
}

#[test]
fn bnd2() {
    let mut bnd = BND2::empty();
    bnd.files = vec![bnd2::File::new(
        7,
        "test.bin".to_string(),
        b"BND2 test".to_vec(),
    )];

    let round_trip = BND2::from_bytes(bnd.to_bytes().unwrap()).unwrap();
    assert_eq!(round_trip.file_info_flags, bnd.file_info_flags);
    assert_eq!(round_trip.file_path_mode as u8, bnd.file_path_mode as u8);
    assert_eq!(round_trip.files[0].id, 7);
    assert_eq!(round_trip.files[0].name, "test.bin");
    assert_eq!(round_trip.files[0].bytes, b"BND2 test");
}

#[test]
fn bnd2_no_names() {
    let mut bnd = BND2::with_path_mode(bnd2::FilePathMode::Nameless);
    bnd.file_info_flags =
        bnd2::FileInfoFlags::ID | bnd2::FileInfoFlags::Offset | bnd2::FileInfoFlags::Size;
    bnd.files = vec![bnd2::File::new(9, "test.bin".to_string(), vec![1, 2, 3])];

    let round_trip = BND2::from_bytes(bnd.to_bytes().unwrap()).unwrap();
    assert_eq!(round_trip.files[0].id, 9);
    assert!(round_trip.files[0].name.is_empty());
    assert_eq!(round_trip.files[0].bytes, vec![1, 2, 3]);
}

#[test]
fn bnd3() {
    println!("Note this BND3 is Kraken compressed, Oodle required");
    let dcx = DCX::from_file("./tests/files/bnd/bnd3.dcx").unwrap();
    let bnd = BND3::from_bytes(dcx.data).unwrap();
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

    for (file, round_trip_file) in bnd.files.iter().zip(round_trip.files.iter()) {
        assert_eq!(file.flags, round_trip_file.flags);
        assert_eq!(file.id, round_trip_file.id);
        assert_eq!(file.name, round_trip_file.name);
        assert_eq!(file.bytes, round_trip_file.bytes);
        assert_eq!(file.compression, round_trip_file.compression);
    }
}

#[test]
fn bnd4() {
    let dcx = DCX::from_file("./tests/files/bnd/bnd4.dcx").unwrap();
    let bnd = BND4::from_bytes(dcx.data).unwrap();
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

    for (file, round_trip_file) in bnd.files.iter().zip(round_trip.files.iter()) {
        assert_eq!(file.flags, round_trip_file.flags);
        assert_eq!(file.id, round_trip_file.id);
        assert_eq!(file.name, round_trip_file.name);
        assert_eq!(file.bytes, round_trip_file.bytes);
        assert_eq!(file.compression, round_trip_file.compression);
    }
}
