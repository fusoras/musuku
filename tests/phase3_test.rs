// We just test if sandbox.rs exists and works
#[test]
fn test_sandbox_generation() {
    let settings = musu::config::Settings::default();
    let path = std::path::PathBuf::from("/tmp/bin");
    let cmd = musu::sandbox::generate_sandbox_exec(&settings, Some(true), None, &path);
    assert!(cmd.contains("bwrap") || cmd.contains("firejail") || cmd == "/tmp/bin");
}
