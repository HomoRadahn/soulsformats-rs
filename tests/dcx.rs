use soulsformats_rs::DCX;
use soulsformats_rs::dcx::compression_info::*;

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
        CompressionInfo::DcxDflt(
            DcxDfltArgs::from_preset(DcxDfltPreset::DcxDflt10000_24_9)
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
