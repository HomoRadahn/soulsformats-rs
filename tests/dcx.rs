use soulsformats_rs::dcx::compression_info::{Type};
use soulsformats_rs::dcx::{DCX};
use soulsformats_rs::io::{BinaryWriter, Endian};

#[test]
fn is() {
    assert!(DCX::is_bytes(b"DCX\0".to_vec()).unwrap());
    assert!(DCX::is_bytes(b"DCP\0".to_vec()).unwrap());
    let dcx_magic = std::env::temp_dir().join(format!(
        "dcx_magic{}.dcx",
        std::process::id()
    ));
    let dcp_magic = std::env::temp_dir().join(format!(
        "dcx_magic{}.dcx",
        std::process::id()
    ));

    let mut writer = BinaryWriter::to_file(&dcx_magic, Endian::Big, true).unwrap();
    writer.write_u8_vec(b"DCX\0".to_vec()).unwrap();
    assert!(DCX::is_file(dcx_magic.to_str().unwrap().to_string()).unwrap());
    writer.finalize().unwrap();
    drop(writer);

    let mut writer_dcp = BinaryWriter::to_file(&dcp_magic, Endian::Big, true).unwrap();
    writer_dcp.write_u8_vec(b"DCP\0".to_vec()).unwrap();
    assert!(DCX::is_file(dcp_magic.to_str().unwrap().to_string()).unwrap());
    writer_dcp.finalize().unwrap();
    drop(writer_dcp);
}

#[test]
fn dcp_dflt() {
    let dcx = DCX::decompress_file("./tests/files/dcp_dflt.dcx".into()).unwrap();
    assert_eq!(dcx.compression.get_type(), Type::DcpDflt);
}

#[test]
fn dcp_edge() {
    let dcx = DCX::decompress_file("./tests/files/dcp_edge.dcx".into()).unwrap();
    assert_eq!(dcx.compression.get_type(), Type::DcpEdge);
}

#[test]
fn dcx_edge() {
    let dcx = DCX::decompress_file("./tests/files/dcx_edge.dcx".into()).unwrap();
    assert_eq!(dcx.compression.get_type(), Type::DcxEdge);
}