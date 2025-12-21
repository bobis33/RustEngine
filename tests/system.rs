use rust_engine::SystemInfo;

#[test]
fn system_info_can_be_created() {
    let info = SystemInfo::new();
    assert!(!info.os.is_empty());
    assert!(!info.arch.is_empty());
}

#[test]
fn unix_or_windows() {
    let info = SystemInfo::new();
    assert!(info.is_windows() || info.is_unix());
}
