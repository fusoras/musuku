use clap_complete::Shell;
use musu::completions::{detect_shell, get_completion_path, install_completions_to};
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
    assert_eq!(zsh_path, home.join(".local/share/zsh/site-functions/_musu"));

    let bash_path = get_completion_path(Shell::Bash, home);
    assert_eq!(
        bash_path,
        home.join(".local/share/bash-completion/completions/musu")
    );

    let fish_path = get_completion_path(Shell::Fish, home);
    assert_eq!(fish_path, home.join(".config/fish/completions/musu.fish"));
}

#[test]
fn test_install_completions_writes_valid_script() {
    let temp = tempdir().unwrap();
    let home = temp.path();

    let path = install_completions_to(Shell::Zsh, home).unwrap();
    assert!(path.exists());
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("#compdef musu"));
    assert!(content.contains("_musu"));
}

#[test]
fn test_zsh_completions_offer_standard_categories_for_categories_flag() {
    let temp = tempdir().unwrap();
    let home = temp.path();

    let path = install_completions_to(Shell::Zsh, home).unwrap();
    let content = std::fs::read_to_string(&path).unwrap();

    let lines: Vec<&str> = content.lines().collect();
    let start = lines
        .iter()
        .position(|line| line.contains("--categories=["))
        .expect("expected a --categories completion line");

    // clap_complete emits one candidate per line; the block ends when the
    // zsh spec closes with `))'`.
    let block: String = lines[start..]
        .iter()
        .take_while(|line| !line.trim_end().ends_with("))'"))
        .copied()
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        !block.ends_with("_default"),
        "categories completion must offer candidates, not fall back to files; got: {block}"
    );
    for candidate in ["AudioVideo", "Development", "Game", "Graphics", "Utility"] {
        assert!(
            block.contains(candidate),
            "expected {candidate} as a completion candidate; got: {block}"
        );
    }
}

#[test]
fn test_fish_completions_offer_standard_categories_for_categories_flag() {
    let temp = tempdir().unwrap();
    let home = temp.path();

    let path = install_completions_to(Shell::Fish, home).unwrap();
    let content = std::fs::read_to_string(&path).unwrap();

    let lines: Vec<&str> = content.lines().collect();
    let start = lines
        .iter()
        .position(|line| line.contains("-l categories"))
        .expect("expected a --categories completion line");

    // clap_complete emits one candidate per line; the block ends when the
    // quoted candidate list closes with `"`.
    let end = lines[start..]
        .iter()
        .position(|line| line.trim_end().ends_with('"'))
        .map(|offset| start + offset + 1)
        .unwrap_or(lines.len());
    let block = lines[start..end].join("\n");

    assert!(
        block.contains("-a \""),
        "fish categories completion must list candidates; got: {block}"
    );
    for candidate in ["AudioVideo", "Game", "Utility"] {
        assert!(
            block.contains(candidate),
            "expected {candidate} as a completion candidate; got: {block}"
        );
    }
}

#[test]
fn test_completion_command_keeps_custom_categories_accepted_by_real_cli() {
    use clap::Parser;
    // The completion-only command gains candidate values, but the real CLI
    // parser must keep accepting custom/free-form categories.
    let result = musu::cli::Cli::try_parse_from([
        "musu",
        "add",
        "./app",
        "--categories",
        "MyCustomCategory,Game",
    ]);
    assert!(
        result.is_ok(),
        "real CLI must not reject custom categories: {:?}",
        result.err()
    );
}

#[test]
fn test_generate_carapace_spec_creates_valid_yaml() {
    let temp = tempdir().unwrap();
    let home = temp.path();

    let path = musu::completions::install_carapace_spec(home).unwrap();
    assert!(path.exists());
    assert_eq!(path, home.join(".config/carapace/specs/musu.yaml"));
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("name: musu"));
    assert!(content.contains("name: add"));
    assert!(content.contains("AudioVideo"));
}

#[test]
fn test_ensure_zshrc_source_line_appends_bun_style_line() {
    let temp = tempdir().unwrap();
    let zshrc = temp.path().join(".zshrc");
    std::fs::write(&zshrc, "# existing zsh config\nexport FOO=bar\n").unwrap();

    let added = musu::completions::ensure_zshrc_source_line(&zshrc).unwrap();
    assert!(added, "Should have added line on first execution");

    let content = std::fs::read_to_string(&zshrc).unwrap();
    assert!(content.contains(r#"[ -s "$HOME/.local/share/zsh/site-functions/_musu" ] && source "$HOME/.local/share/zsh/site-functions/_musu""#));

    // Calling again must be idempotent
    let added_second = musu::completions::ensure_zshrc_source_line(&zshrc).unwrap();
    assert!(!added_second, "Should not add duplicate line");
}

#[test]
fn test_is_carapace_installed_in_paths() {
    let temp = tempdir().unwrap();
    let empty_bin = temp.path().join("empty_bin");
    std::fs::create_dir_all(&empty_bin).unwrap();

    // With no carapace binary in dir, should be false
    assert!(!musu::completions::is_carapace_in_dirs(&[&empty_bin]));

    // When carapace binary is present and executable
    let fake_carapace = empty_bin.join("carapace");
    std::fs::write(&fake_carapace, "#!/bin/sh\nexit 0\n").unwrap();
    assert!(musu::completions::is_carapace_in_dirs(&[&empty_bin]));
}

#[test]
fn test_is_dir_in_path_list() {
    let temp = tempdir().unwrap();
    let bin1 = temp.path().join("bin1");
    let bin2 = temp.path().join("bin2");
    std::fs::create_dir_all(&bin1).unwrap();
    std::fs::create_dir_all(&bin2).unwrap();

    let paths = vec![bin1.clone()];
    assert!(musu::completions::is_dir_in_paths(&bin1, &paths));
    assert!(!musu::completions::is_dir_in_paths(&bin2, &paths));
}

#[test]
fn test_format_install_notice_not_in_path() {
    let bin_path = std::path::Path::new("/home/user/.local/bin/musu");
    let output = musu::completions::format_install_notice(
        bin_path,
        clap_complete::Shell::Zsh,
        false,
        Some(std::path::Path::new("/home/user")),
    );
    assert!(output.contains(
        "######################################################################## 100.0%"
    ));
    assert!(output.contains("musu was installed successfully to ~/.local/bin/musu"));
    assert!(output.contains("Manually add the directory to ~/.zshrc (or similar):"));
    assert!(output.contains("  export PATH=\"$HOME/.local/bin:$PATH\""));
    assert!(output.contains("To get started, run:\n\n  musu --help"));
}

#[test]
fn test_format_install_notice_already_in_path() {
    let bin_path = std::path::Path::new("/home/user/.local/bin/musu");
    let output = musu::completions::format_install_notice(
        bin_path,
        clap_complete::Shell::Zsh,
        true,
        Some(std::path::Path::new("/home/user")),
    );
    assert!(output.contains(
        "######################################################################## 100.0%"
    ));
    assert!(output.contains("musu was installed successfully to ~/.local/bin/musu"));
    assert!(!output.contains("Manually add the directory"));
    assert!(output.contains("To get started, run:\n\n  musu --help"));
}

#[test]
fn test_format_manual_completion_notice() {
    let output = musu::completions::format_manual_completion_notice(clap_complete::Shell::Zsh);
    assert!(output.contains("Manually add the completion to ~/.zshrc (or similar):"));
    assert!(output.contains(
        r#"[ -s "$HOME/.local/share/zsh/site-functions/_musu" ] && source "$HOME/.local/share/zsh/site-functions/_musu""#
    ));
}
