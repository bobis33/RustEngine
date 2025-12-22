use rust_engine::SystemInfo;

#[test]
fn system_info_can_be_created() {
    let info = SystemInfo::new();
    assert!(!info.os_arch.is_empty());
}
