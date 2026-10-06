use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_install_script_not_in_path_outputs_bun_style_notice() {
    let temp = tempdir().unwrap();
    let install_dir = temp.path().join("bin");

    let script_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("install.sh");

    // Run install.sh with GEICO_INSTALL_DIR pointing to our temp bin dir
    // and a restricted PATH that doesn't contain install_dir
    let output = Command::new("sh")
        .arg(&script_path)
        .env("GEICO_INSTALL_DIR", &install_dir)
        .env("PATH", "/usr/bin:/bin")
        .env("SHELL", "/usr/bin/zsh")
        .env("HOME", temp.path())
        .output()
        .expect("Failed to execute install.sh");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "install.sh failed with exit code {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        stdout,
        stderr
    );

    let installed_bin = install_dir.join("geico");
    assert!(
        installed_bin.exists(),
        "Binary should have been installed to {}",
        installed_bin.display()
    );

    assert!(
        stdout.contains(
            "######################################################################## 100.0%"
        ),
        "Missing progress bar in stdout: {}",
        stdout
    );
    assert!(
        stdout.contains("geico was installed successfully to"),
        "Missing success message in stdout: {}",
        stdout
    );
    assert!(
        stdout.contains("Manually add the directory to ~/.zshrc (or similar):"),
        "Missing manual PATH notice in stdout: {}",
        stdout
    );
    assert!(
        stdout.contains("export PATH="),
        "Missing export PATH line in stdout: {}",
        stdout
    );
    assert!(
        stdout.contains("To get started, run:\n\n  geico --help"),
        "Missing getting started line in stdout: {}",
        stdout
    );
}

#[test]
fn test_install_script_when_already_in_path_skips_manual_path_notice() {
    let temp = tempdir().unwrap();
    let install_dir = temp.path().join("bin");
    std::fs::create_dir_all(&install_dir).unwrap();

    let script_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("install.sh");

    let path_with_install_dir = format!("{}:/usr/bin:/bin", install_dir.display());

    let output = Command::new("sh")
        .arg(&script_path)
        .env("GEICO_INSTALL_DIR", &install_dir)
        .env("PATH", path_with_install_dir)
        .env("SHELL", "/usr/bin/zsh")
        .env("HOME", temp.path())
        .output()
        .expect("Failed to execute install.sh");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("geico was installed successfully to"));
    assert!(
        !stdout.contains("Manually add the directory to"),
        "Should not ask to manually add directory when it is already in PATH: {}",
        stdout
    );
    assert!(stdout.contains("To get started, run:\n\n  geico --help"));
}
