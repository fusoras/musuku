//! Desktop `Exec=` formatting must follow the Desktop Entry spec: quote
//! individual arguments with double quotes, never the whole line in single
//! quotes (single quotes are reserved characters and invalidate the entry,
//! making launchers disappear from menus).

use musu::desktop::{self, Comments};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn install_with(exec: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let temp = tempdir().unwrap();
    let (file, _) = desktop::install(
        &temp.path().join("share"),
        "test-app",
        "Test App",
        exec,
        None,
        None,
        Comments::default(),
    )
    .unwrap();
    (temp, file)
}

fn exec_line(file: &Path) -> String {
    fs::read_to_string(file)
        .unwrap()
        .lines()
        .find_map(|line| line.strip_prefix("Exec=").map(str::to_string))
        .unwrap()
}

#[test]
fn test_exec_command_is_never_wrapped_in_single_quotes() {
    // A multi-word command line (the sandbox wrapper) must be written
    // verbatim — wrapping the whole value in single quotes is what broke
    // real launchers: reserved characters invalidated them and desktop
    // shells dropped the entries.
    let exec = "bwrap --unshare-all --ro-bind / / /opt/app.AppImage";
    let (_temp, file) = install_with(exec);
    let line = exec_line(&file);
    assert_eq!(line, exec);
    assert!(
        !line.contains('\''),
        "single quotes are reserved in Exec values: {line}"
    );
}

#[test]
fn test_exec_quotes_tokens_with_reserved_characters() {
    // `'` and `$` inside a token: the token gets double-quoted, `$` is
    // backslash-escaped, `'` stays literal (legal inside double quotes).
    let exec = "/opt/tool --message it's$dangerous";
    let (_temp, file) = install_with(exec);
    let line = exec_line(&file);
    assert_eq!(line, "/opt/tool --message \"it's\\$dangerous\"");
}

#[test]
fn test_exec_plain_path_is_written_verbatim() {
    let (_temp, file) = install_with("/opt/zen/zen");
    assert_eq!(exec_line(&file), "/opt/zen/zen");
}

#[test]
fn test_freshly_installed_exec_is_not_out_of_date() {
    let exec = "bwrap --unshare-all /home/u/My Apps/app.AppImage";
    let (_temp, file) = install_with(exec);
    assert!(
        !desktop::exec_out_of_date(&file, exec),
        "a launcher just written must compare as up to date"
    );
}
