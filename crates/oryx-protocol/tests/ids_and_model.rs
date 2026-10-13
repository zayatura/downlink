use oryx_protocol::{LayoutIds, Model};

#[test]
fn parses_layout_and_revision() {
    let ids = LayoutIds::parse("eVAVm/m57mrj").expect("valid ids");
    assert_eq!(ids.layout_id(), "eVAVm");
    assert_eq!(ids.revision_id(), "m57mrj");
    assert_eq!(ids.to_string(), "eVAVm/m57mrj");
}

#[test]
fn surrounding_whitespace_is_ignored() {
    assert_eq!(
        LayoutIds::parse(" abc/def \n"),
        LayoutIds::new("abc", "def")
    );
}

#[test]
fn rejects_anything_unsafe_for_a_file_name() {
    for bad in [
        "",
        "abc",
        "/def",
        "abc/",
        "../../etc",
        "abc/../x",
        "a b/c",
        "abc/def/ghi",
        "a.b/c",
        "ab\\c/d",
        "été/x",
        "abcdefghijklmnopqrstuvwxyz0123456/x", // 33 characters
    ] {
        assert_eq!(LayoutIds::parse(bad), None, "{bad:?} must be rejected");
    }
}

#[test]
fn accepts_32_character_parts() {
    let long = "a".repeat(32);
    assert!(LayoutIds::new(&long, &long).is_some());
}

#[test]
fn voyager_is_recognised_only_on_its_raw_hid_interface() {
    assert_eq!(Model::from_ids(0x3297, 0x1977), Some(Model::Voyager));
    assert_eq!(Model::from_ids(0x3297, 0x0001), None);
    assert_eq!(
        Model::from_interface(0x3297, 0x1977, 0xFF60, 0x61),
        Some(Model::Voyager)
    );
    assert_eq!(Model::from_interface(0x3297, 0x1977, 0x0001, 0x06), None);
    assert_eq!(Model::Voyager.name(), "Voyager");
    assert_eq!(Model::Voyager.oryx_geometry(), "voyager");
}
