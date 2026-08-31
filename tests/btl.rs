use soulsformats_rs::BTL;

#[test]
fn read_btl() {
    let _btl = BTL::from_file("./tests/files/btl/btl.dcx").unwrap();
}