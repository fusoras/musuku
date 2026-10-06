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
