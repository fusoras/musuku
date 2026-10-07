// We just test if sandbox.rs exists and works
#[test]
fn test_sandbox_generation() {
    let settings = geico::config::Settings::default();
    let path = std::path::PathBuf::from("/tmp/bin");
    let cmd = geico::sandbox::generate_sandbox_exec(&settings, Some(true), &path);
    assert!(cmd.contains("bwrap") || cmd.contains("firejail") || cmd == "/tmp/bin");
}
