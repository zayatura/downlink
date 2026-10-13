use downlink::single_instance::acquire;

#[test]
fn a_second_lock_is_refused_until_the_first_is_released() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested").join("downlink.lock");
    let first = acquire(&path).unwrap().expect("first lock");
    assert!(
        acquire(&path).unwrap().is_none(),
        "second instance must be refused"
    );
    drop(first);
    assert!(
        acquire(&path).unwrap().is_some(),
        "free again after release"
    );
}
