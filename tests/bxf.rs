use soulsformats_rs::{
    binder::{BXF3, BXF4, BinderFile, FileFlags},
    dcx::compression_info::CompressionInfo,
};

#[test]
fn bxf3_round_trip() {
    let mut bxf = BXF3::new(CompressionInfo::DcpDflt, CompressionInfo::DcpDflt);
    bxf.files = vec![
        BinderFile::new(
            FileFlags::None,
            10,
            "first.bin".to_string(),
            b"BXF3 first file".to_vec(),
            CompressionInfo::DcpDflt,
        ),
        BinderFile::new(
            FileFlags::Compressed,
            20,
            "second.bin".to_string(),
            b"BXF3 second file with per-file compression".to_vec(),
            CompressionInfo::DcpDflt,
        ),
    ];

    let (bhd_bytes, bdt_bytes) = bxf.to_bytes().unwrap();
    assert!(BXF3::is_header_bytes(bhd_bytes.clone()).unwrap());
    assert!(BXF3::is_data_bytes(bdt_bytes.clone()).unwrap());
    let round_trip = BXF3::from_bytes(bhd_bytes, bdt_bytes).unwrap();

    assert_eq!(
        bxf.version.trim_end_matches('\0'),
        round_trip.version.trim_end_matches('\0')
    );
    assert_eq!(bxf.format, round_trip.format);
    assert_eq!(bxf.bhd_compression, round_trip.bhd_compression);
    assert_eq!(bxf.bdt_compression, round_trip.bdt_compression);
    assert_eq!(bxf.files.len(), round_trip.files.len());
    for (file, round_trip_file) in bxf.files.iter().zip(round_trip.files.iter()) {
        assert_eq!(file.flags, round_trip_file.flags);
        assert_eq!(file.id, round_trip_file.id);
        assert_eq!(file.name, round_trip_file.name);
        assert_eq!(file.bytes, round_trip_file.bytes);
        if file.flags.contains(FileFlags::Compressed) {
            assert_eq!(file.compression, round_trip_file.compression);
        } else {
            assert_eq!(CompressionInfo::Zlib, round_trip_file.compression);
        }
    }
}

#[test]
fn bxf4_round_trip() {
    let mut bxf = BXF4::new(CompressionInfo::DcpDflt, CompressionInfo::DcpDflt);
    bxf.files = vec![BinderFile::new(
        FileFlags::None,
        40,
        "bxf4.bin".to_string(),
        b"BXF4 in memory".to_vec(),
        CompressionInfo::DcpDflt,
    )];

    let (bhd_bytes, bdt_bytes) = bxf.to_bytes().unwrap();
    assert!(BXF4::is_header_bytes(bhd_bytes.clone()).unwrap());
    assert!(BXF4::is_data_bytes(bdt_bytes.clone()).unwrap());
    let round_trip = BXF4::from_bytes(bhd_bytes, bdt_bytes).unwrap();

    assert_eq!(bxf.format, round_trip.format);
    assert_eq!(bxf.unicode, round_trip.unicode);
    assert_eq!(bxf.extended, round_trip.extended);
    assert_eq!(bxf.files[0].id, round_trip.files[0].id);
    assert_eq!(bxf.files[0].name, round_trip.files[0].name);
    assert_eq!(bxf.files[0].bytes, round_trip.files[0].bytes);
}
