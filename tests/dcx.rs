use soulsformats_rs::DCX;
use soulsformats_rs::dcx::compression_info::*;
use soulsformats_rs::io::{BinaryWriter, Endian};

#[test]
fn is() {
    assert!(DCX::is_bytes(b"DCX\0".to_vec()).unwrap());
    assert!(DCX::is_bytes(b"DCP\0".to_vec()).unwrap());
    let dcx_magic = std::env::temp_dir().join(format!("dcx_magic{}.dcx", std::process::id()));
    let dcp_magic = std::env::temp_dir().join(format!("dcx_magic{}.dcx", std::process::id()));

    let mut writer = BinaryWriter::to_file(&dcx_magic, Endian::Big, false).unwrap();
    writer.write_u8_vec(b"DCX\0".to_vec()).unwrap();
    assert!(DCX::is_file(dcx_magic.to_str().unwrap()).unwrap());
    writer.finalize().unwrap();
    drop(writer);

    let mut writer_dcp = BinaryWriter::to_file(&dcp_magic, Endian::Big, false).unwrap();
    writer_dcp.write_u8_vec(b"DCP\0".to_vec()).unwrap();
    assert!(DCX::is_file(dcp_magic.to_str().unwrap()).unwrap());
    writer_dcp.finalize().unwrap();
    drop(writer_dcp);
}

#[test]
fn dcp_dflt_round_trip() {
    let dcx = DCX::new(
        b"hello, soulsformats-rs".to_vec(),
        CompressionInfo::DcpDflt,
    );

    let output = dcx.compress_to_bytes().unwrap();

    let round_trip = DCX::decompress_bytes(output).unwrap();

    assert_eq!(round_trip.decompressed, b"hello, soulsformats-rs".to_vec());
}

#[test]
fn dcp_edge_round_trip() {
    let dcx = DCX::new(
        b"hello, soulsformats-rs".to_vec(),
        CompressionInfo::DcpEdge,
    );

    let output = dcx.compress_to_bytes().unwrap();

    let round_trip = DCX::decompress_bytes(output).unwrap();

    assert_eq!(round_trip.decompressed, b"hello, soulsformats-rs".to_vec());
}

#[test]
fn dcx_edge_round_trip() {
    let dcx = DCX::new(
        b"hello, soulsformats-rs".to_vec(),
        CompressionInfo::DcxEdge,
    );

    let output = dcx.compress_to_bytes().unwrap();

    let round_trip = DCX::decompress_bytes(output).unwrap();

    assert_eq!(round_trip.decompressed, b"hello, soulsformats-rs".to_vec());
}

#[test]
fn dcx_dftl_round_trip() {
    let dcx = DCX::new(
        b"hello, soulsformats-rs".to_vec(),
        CompressionInfo::dcx_dflt_from_preset(
            DcxDfltCompressionPreset::DcxDflt10000_24_9,
        ),
    );

    let output = dcx.compress_to_bytes().unwrap();

    let round_trip = DCX::decompress_bytes(output).unwrap();

    assert_eq!(round_trip.decompressed, b"hello, soulsformats-rs".to_vec());
}

#[test]
#[should_panic]
fn dcx_krak_round_trip() {
    let dcx = DCX::new(
        b"hello, soulsformats-rs".to_vec(),
        CompressionInfo::DcxKrak,
    );

    dcx.compress_to_bytes().unwrap();
}

#[test]
fn dcx_zstd_round_trip() {
    let dcx = DCX::new(
        b"hello, soulsformats-rs".to_vec(),
        CompressionInfo::DcxZstd(6)
    );

    let output = dcx.compress_to_bytes().unwrap();

    let round_trip = DCX::decompress_bytes(output).unwrap();

    assert_eq!(round_trip.decompressed, b"hello, soulsformats-rs".to_vec());
}
