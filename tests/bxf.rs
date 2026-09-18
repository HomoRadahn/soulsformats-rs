use soulsformats_rs::{
    binder::{BXF3, BinderFile, FileFlags},
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
