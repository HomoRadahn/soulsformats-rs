use soulsformats_rs::Dcx;
use soulsformats_rs::dcx::compression_info::*;

#[test]
fn dcp_dflt() {
    let dcx = Dcx::new(b"hello, soulsformats-rs".to_vec(), CompressionInfo::DcpDflt);

    let output = dcx.to_bytes().unwrap();

    let round_trip = Dcx::from_bytes(output).unwrap();

    assert_eq!(round_trip.data, b"hello, soulsformats-rs".to_vec());
}

#[test]
fn dcp_edge() {
    let dcx = Dcx::new(b"hello, soulsformats-rs".to_vec(), CompressionInfo::DcpEdge);

    let output = dcx.to_bytes().unwrap();

    let round_trip = Dcx::from_bytes(output).unwrap();

    assert_eq!(round_trip.data, b"hello, soulsformats-rs".to_vec());
}

#[test]
fn dcx_edge() {
    let dcx = Dcx::new(b"hello, soulsformats-rs".to_vec(), CompressionInfo::DcxEdge);

    let output = dcx.to_bytes().unwrap();

    let round_trip = Dcx::from_bytes(output).unwrap();

    assert_eq!(round_trip.data, b"hello, soulsformats-rs".to_vec());
}

#[test]
fn dcx_dftl() {
    let dcx = Dcx::new(
        b"hello, soulsformats-rs".to_vec(),
        CompressionInfo::DcxDflt(DcxDfltArgs::from_preset(DcxDfltPreset::DcxDflt10000_24_9)),
    );

    let output = dcx.to_bytes().unwrap();

    let round_trip = Dcx::from_bytes(output).unwrap();

    assert_eq!(round_trip.data, b"hello, soulsformats-rs".to_vec());
}

#[test]
fn dcx_krak() {
    let dcx = Dcx::new(
        b"hello, soulsformats-rs".to_vec(),
        CompressionInfo::DcxKrak(DcxKrakArgs::new()),
    );

    let output = dcx.to_bytes().unwrap();
    let round_trip = Dcx::from_bytes(output).unwrap();

    assert_eq!(round_trip.data, b"hello, soulsformats-rs".to_vec());
}

#[test]
fn dcx_zstd() {
    let dcx = Dcx::new(
        b"hello, soulsformats-rs".to_vec(),
        CompressionInfo::DcxZstd(6),
    );

    let output = dcx.to_bytes().unwrap();

    let round_trip = Dcx::from_bytes(output).unwrap();

    assert_eq!(round_trip.data, b"hello, soulsformats-rs".to_vec());
}
