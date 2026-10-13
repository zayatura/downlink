#![cfg(feature = "hid")]

use oryx_device::describe_open_error;

#[test]
fn exclusive_access_points_at_keymapp() {
    let raw = "hid_open_path: failed to open IOHIDDevice from mach entry: (0xE00002C5) (iokit/common) exclusive access and device already open";
    assert_eq!(
        describe_open_error(raw),
        "The keyboard is in use by another app (quit Keymapp)"
    );
}

#[test]
fn other_errors_pass_through_unchanged() {
    assert_eq!(
        describe_open_error("permission denied"),
        "permission denied"
    );
}
