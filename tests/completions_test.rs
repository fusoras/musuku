use clap_complete::Shell;
use geico::completions::{detect_shell, get_completion_path, install_completions_to};
use tempfile::tempdir;

#[test]
fn test_detect_shell_from_env() {
    assert_eq!(detect_shell(Some("/usr/bin/zsh")), Shell::Zsh);
    assert_eq!(detect_shell(Some("/bin/bash")), Shell::Bash);
    assert_eq!(detect_shell(Some("/usr/bin/fish")), Shell::Fish);
    assert_eq!(detect_shell(Some("zsh")), Shell::Zsh);
    assert_eq!(detect_shell(None), Shell::Bash);
}

#[test]
fn test_get_completion_path_for_shells() {
    let temp = tempdir().unwrap();
    let home = temp.path();

    let zsh_path = get_completion_path(Shell::Zsh, home);
    assert_eq!(
        zsh_path,
        home.join(".local/share/zsh/site-functions/_geico")
    );

    let bash_path = get_completion_path(Shell::Bash, home);
    assert_eq!(
        bash_path,
        home.join(".local/share/bash-completion/completions/geico")
    );

    let fish_path = get_completion_path(Shell::Fish, home);
    assert_eq!(fish_path, home.join(".config/fish/completions/geico.fish"));
}

#[test]
fn test_install_completions_writes_valid_script() {
    let temp = tempdir().unwrap();
    let home = temp.path();

    let path = install_completions_to(Shell::Zsh, home).unwrap();
    assert!(path.exists());
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("#compdef geico"));
    assert!(content.contains("_geico"));
}

#[test]
fn test_generate_carapace_spec_creates_valid_yaml() {
    let temp = tempdir().unwrap();
    let home = temp.path();

    let path = geico::completions::install_carapace_spec(home).unwrap();
    assert!(path.exists());
    assert_eq!(path, home.join(".config/carapace/specs/geico.yaml"));
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("name: geico"));
    assert!(content.contains("name: add"));
    assert!(content.contains("AudioVideo"));
}

#[test]
fn test_ensure_zshrc_source_line_appends_bun_style_line() {
    let temp = tempdir().unwrap();
    let zshrc = temp.path().join(".zshrc");
    std::fs::write(&zshrc, "# existing zsh config\nexport FOO=bar\n").unwrap();

    let added = geico::completions::ensure_zshrc_source_line(&zshrc).unwrap();
    assert!(added, "Should have added line on first execution");

    let content = std::fs::read_to_string(&zshrc).unwrap();
    assert!(content.contains(r#"[ -s "$HOME/.local/share/zsh/site-functions/_geico" ] && source "$HOME/.local/share/zsh/site-functions/_geico""#));

    // Calling again must be idempotent
    let added_second = geico::completions::ensure_zshrc_source_line(&zshrc).unwrap();
    assert!(!added_second, "Should not add duplicate line");
}

#[test]
fn test_is_carapace_installed_in_paths() {
    let temp = tempdir().unwrap();
    let empty_bin = temp.path().join("empty_bin");
    std::fs::create_dir_all(&empty_bin).unwrap();

    // With no carapace binary in dir, should be false
    assert!(!geico::completions::is_carapace_in_dirs(&[&empty_bin]));

    // When carapace binary is present and executable
    let fake_carapace = empty_bin.join("carapace");
    std::fs::write(&fake_carapace, "#!/bin/sh\nexit 0\n").unwrap();
    assert!(geico::completions::is_carapace_in_dirs(&[&empty_bin]));
}

#[test]
fn test_is_dir_in_path_list() {
    let temp = tempdir().unwrap();
    let bin1 = temp.path().join("bin1");
    let bin2 = temp.path().join("bin2");
    std::fs::create_dir_all(&bin1).unwrap();
    std::fs::create_dir_all(&bin2).unwrap();

    let paths = vec![bin1.clone()];
    assert!(geico::completions::is_dir_in_paths(&bin1, &paths));
    assert!(!geico::completions::is_dir_in_paths(&bin2, &paths));
}

#[test]
fn test_format_install_notice_not_in_path() {
    let bin_path = std::path::Path::new("/home/user/.local/bin/geico");
    let output = geico::completions::format_install_notice(
        bin_path,
        clap_complete::Shell::Zsh,
        false,
        Some(std::path::Path::new("/home/user")),
    );
    assert!(output.contains(
        "######################################################################## 100.0%"
    ));
    assert!(output.contains("geico was installed successfully to ~/.local/bin/geico"));
    assert!(output.contains("Manually add the directory to ~/.zshrc (or similar):"));
    assert!(output.contains("  export PATH=\"$HOME/.local/bin:$PATH\""));
    assert!(output.contains("To get started, run:\n\n  geico --help"));
}

#[test]
fn test_format_install_notice_already_in_path() {
    let bin_path = std::path::Path::new("/home/user/.local/bin/geico");
    let output = geico::completions::format_install_notice(
        bin_path,
        clap_complete::Shell::Zsh,
        true,
        Some(std::path::Path::new("/home/user")),
    );
    assert!(output.contains(
        "######################################################################## 100.0%"
    ));
    assert!(output.contains("geico was installed successfully to ~/.local/bin/geico"));
    assert!(!output.contains("Manually add the directory"));
    assert!(output.contains("To get started, run:\n\n  geico --help"));
}

#[test]
fn test_format_manual_completion_notice() {
    let output = geico::completions::format_manual_completion_notice(clap_complete::Shell::Zsh);
    assert!(output.contains("Manually add the completion to ~/.zshrc (or similar):"));
    assert!(output.contains(
        r#"[ -s "$HOME/.local/share/zsh/site-functions/_geico" ] && source "$HOME/.local/share/zsh/site-functions/_geico""#
    ));
}
