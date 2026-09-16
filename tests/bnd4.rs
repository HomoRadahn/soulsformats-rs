use soulsformats_rs::{BND4, SoulsFile};

#[test]
fn bnd4_read() {
    let bnd = BND4::from_file("./tests/files/bnd/test.bnd.dcx").unwrap();
    let text1 = String::from_utf8(bnd.files[0].bytes.clone()).unwrap();
    let text2 = String::from_utf8(bnd.files[1].bytes.clone()).unwrap();
    assert_eq!(text1, "BND4 Test");
    assert_eq!(text2, "BND4 Test 2");
}
