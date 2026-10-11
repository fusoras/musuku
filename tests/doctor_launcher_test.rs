//! `musu doctor` sandbox-launcher staleness detection and `--fix`
//! regeneration: detection after a global policy change, regeneration of
//! the `.desktop` launcher and the `$PATH` wrapper, idempotency, per-app
//! policy override preservation, and the skip rules (non-AppImage targets,
//! no `.desktop`, sandbox off). Staleness detection and regeneration only
//! apply to AppImages.

use musu::config::{self, SandboxPolicy, Settings};
use musu::doctor::run_doctor;
use musu::manager::{AddOptions, Manager};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn executable(path: &Path) {
    fs::write(path, "#!/bin/sh\necho hi\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

/// Fake AppImage target, following the repo-wide test pattern (see
/// `tests/integration_test.rs::appimage`). `extractor::is_appimage`
/// accepts either a non-empty file with a case-insensitive `.AppImage`
/// extension (the path taken here) or, for extensionless files, the `AI`
/// magic bytes at offset 0; the shebang body keeps the probe that
/// `auto_extract` runs (`--appimage-extract`) harmless.
fn appimage(path: &Path) {
    executable(path);
}

fn manager_with(base: &Path) -> Manager {
    Manager::new(
        base.join("config/apps.toml"),
        base.join("share"),
        base.join("bin"),
    )
}

/// Sandbox on with the deterministic `bwrap` backend so tests never probe
/// the host for `bwrap`/`firejail`. `symlink_appimages` is on because
/// AppImage targets only get a `$PATH` wrapper when it is enabled, and the
/// wrapper-regeneration tests need one to exist.
fn bwrap_settings() -> Settings {
    let mut settings = Settings::default();
    settings.sandbox.enabled = true;
    settings.sandbox.backend = "bwrap".to_string();
    settings.defaults.symlink_appimages = true;
    settings
}

fn write_settings(base: &Path, settings: &Settings) {
    fs::create_dir_all(base.join("config")).unwrap();
    config::save_settings(&base.join("config"), settings).unwrap();
}

fn add_app(manager: &Manager, base: &Path, name: &str, extra: AddOptions) {
    add_app_as(manager, base, name, extra, true)
}

/// Register `name` with a non-AppImage target (plain shell script) so the
/// AppImage-only scope of the launcher checks can be exercised.
fn add_script_app(manager: &Manager, base: &Path, name: &str, extra: AddOptions) {
    add_app_as(manager, base, name, extra, false)
}

fn add_app_as(manager: &Manager, base: &Path, name: &str, extra: AddOptions, is_app: bool) {
    let extension = if is_app { "AppImage" } else { "target" };
    let target = base.join("bin").join(format!("{name}.{extension}"));
    fs::create_dir_all(base.join("bin")).unwrap();
    if is_app {
        appimage(&target);
    } else {
        executable(&target);
    }
    manager
        .add(AddOptions {
            target,
            name: Some(name.to_string()),
            ..extra
        })
        .unwrap();
}

fn stale_details(report: &musu::doctor::DoctorReport) -> Vec<&String> {
    report
        .details
        .iter()
        .filter(|d| d.contains("out of date"))
        .collect()
}

// ---------------------------------------------------------------------------
// Detection (doctor without --fix)
// ---------------------------------------------------------------------------

/// A sandboxed app whose baked `Exec` no longer matches the effective
/// policy is reported; an app registered under the current policy is not.
#[test]
fn test_doctor_reports_stale_sandbox_launcher_only_for_outdated_exec() {
    let temp = tempdir().unwrap();
    let base = temp.path();
    let manager = manager_with(base);

    // Registered under the default global policy (network allowed).
    write_settings(base, &bwrap_settings());
    add_app(&manager, base, "Stale App", AddOptions::default());

    // The user edits `[sandbox.policy]` in config.toml afterwards.
    let mut changed = bwrap_settings();
    changed.sandbox.policy.network = Some(false);
    write_settings(base, &changed);

    // Registered after the change: its Exec already matches.
    add_app(&manager, base, "Fresh App", AddOptions::default());

    let report = run_doctor(&manager, false, false).unwrap();
    let stale = stale_details(&report);
    assert_eq!(
        stale.len(),
        1,
        "exactly the outdated launcher must be reported, got: {:?}",
        report.details
    );
    assert!(
        stale[0].contains("stale-app"),
        "report must name the stale app id: {}",
        stale[0]
    );
    assert!(
        !report
            .details
            .iter()
            .any(|d| d.contains("fresh-app") && d.contains("out of date")),
        "the fresh app must not be reported as stale"
    );
}

// ---------------------------------------------------------------------------
// Repair (doctor --fix)
// ---------------------------------------------------------------------------

/// `--fix` rewrites the stale `.desktop` file with the newly generated
/// sandbox command, keeping the rest of the launcher intact.
#[test]
fn test_doctor_fix_regenerates_stale_launcher_exec() {
    let temp = tempdir().unwrap();
    let base = temp.path();
    let manager = manager_with(base);

    write_settings(base, &bwrap_settings());
    add_app(&manager, base, "Stale App", AddOptions::default());

    // New global policy adds a hidden path (bwrap renders it as --tmpfs).
    let mut changed = bwrap_settings();
    changed.sandbox.policy.hide_paths = Some(vec!["/musu-doctor-hidden".to_string()]);
    write_settings(base, &changed);

    let before = run_doctor(&manager, false, false).unwrap();
    assert_eq!(stale_details(&before).len(), 1);

    run_doctor(&manager, true, true).unwrap();

    let entry = manager.get("stale-app").unwrap();
    let desktop = fs::read_to_string(entry.desktop_file.as_ref().unwrap()).unwrap();
    assert!(
        desktop.contains("--tmpfs /musu-doctor-hidden"),
        "launcher must embed the new policy command, got:\n{desktop}"
    );
    assert!(
        !desktop.contains("Exec=echo hi"),
        "the sandbox wrapper must still be present"
    );
    // The target binary is never touched (non-destructive invariant).
    assert!(entry.target_bin.exists());
}

/// Running `--fix` twice in a row: after the first pass the Exec values
/// match, so the second pass reports nothing and rewrites nothing.
#[test]
fn test_doctor_fix_is_idempotent() {
    let temp = tempdir().unwrap();
    let base = temp.path();
    let manager = manager_with(base);

    write_settings(base, &bwrap_settings());
    add_app(&manager, base, "Stale App", AddOptions::default());

    let mut changed = bwrap_settings();
    changed.sandbox.policy.network = Some(false);
    write_settings(base, &changed);

    run_doctor(&manager, true, true).unwrap();

    let desktop_path = manager.get("stale-app").unwrap().desktop_file.unwrap();
    let content = fs::read_to_string(&desktop_path).unwrap();
    assert!(
        !content.contains("--share-net"),
        "first fix must have regenerated the launcher without --share-net:\n{content}"
    );
    let mtime = fs::metadata(&desktop_path).unwrap().modified().unwrap();

    let second = run_doctor(&manager, true, true).unwrap();
    assert!(
        stale_details(&second).is_empty(),
        "second run must not report stale launchers: {:?}",
        second.details
    );
    assert_eq!(
        fs::read_to_string(&desktop_path).unwrap(),
        content,
        "second run must not rewrite the launcher"
    );
    assert_eq!(
        fs::metadata(&desktop_path).unwrap().modified().unwrap(),
        mtime,
        "second run must not touch the launcher file"
    );
}

/// A per-app `sandbox_policy` keeps winning over the edited global policy
/// after regeneration: the new global fields appear, the overridden field
/// still reflects the app's own value.
#[test]
fn test_doctor_fix_preserves_per_app_policy_override() {
    let temp = tempdir().unwrap();
    let base = temp.path();
    let manager = manager_with(base);

    write_settings(base, &bwrap_settings());
    add_app(
        &manager,
        base,
        "Override App",
        AddOptions {
            sandbox_policy: Some(SandboxPolicy {
                network: Some(false),
                ..Default::default()
            }),
            ..Default::default()
        },
    );

    // Global policy gains a hidden path; the app keeps its own network rule.
    let mut changed = bwrap_settings();
    changed.sandbox.policy.hide_paths = Some(vec!["/musu-global-hidden".to_string()]);
    write_settings(base, &changed);

    let before = run_doctor(&manager, false, false).unwrap();
    assert_eq!(stale_details(&before).len(), 1);

    run_doctor(&manager, true, true).unwrap();

    let entry = manager.get("override-app").unwrap();
    let desktop = fs::read_to_string(entry.desktop_file.as_ref().unwrap()).unwrap();
    assert!(
        desktop.contains("--tmpfs /musu-global-hidden"),
        "the new global field must be applied:\n{desktop}"
    );
    assert!(
        !desktop.contains("--share-net"),
        "the per-app network=false override must survive regeneration:\n{desktop}"
    );
    // The override itself is still stored in the registry.
    assert_eq!(entry.sandbox_policy.as_ref().unwrap().network, Some(false));
}

// ---------------------------------------------------------------------------
// Skip rules
// ---------------------------------------------------------------------------

/// Apps without a `.desktop` launcher (`--no-desktop`) and apps with the
/// sandbox turned off are never flagged nor regenerated; only sandboxed
/// launchers on disk are candidates.
#[test]
fn test_doctor_skips_apps_without_desktop_or_with_sandbox_off() {
    let temp = tempdir().unwrap();
    let base = temp.path();
    let manager = manager_with(base);

    write_settings(base, &bwrap_settings());
    add_app(
        &manager,
        base,
        "NoDesktop App",
        AddOptions {
            no_desktop: true,
            ..Default::default()
        },
    );
    add_app(
        &manager,
        base,
        "SandboxOff App",
        AddOptions {
            sandbox: Some(false),
            ..Default::default()
        },
    );
    // Sibling that IS stale, proving detection actually runs this pass.
    add_app(&manager, base, "Stale App", AddOptions::default());

    let mut changed = bwrap_settings();
    changed.sandbox.policy.hide_paths = Some(vec!["/musu-doctor-hidden".to_string()]);
    changed.sandbox.policy.network = Some(false);
    write_settings(base, &changed);

    let report = run_doctor(&manager, false, false).unwrap();
    let stale = stale_details(&report);
    assert_eq!(
        stale.len(),
        1,
        "only the sandboxed launcher on disk may be reported: {:?}",
        report.details
    );
    assert!(stale[0].contains("stale-app"));

    // --fix touches nothing for the skipped apps.
    run_doctor(&manager, true, true).unwrap();
    let no_desktop = manager.get("nodesktop-app").unwrap();
    assert!(no_desktop.desktop_file.is_none(), "must stay desktop-less");
    let off = manager.get("sandboxoff-app").unwrap();
    let desktop = fs::read_to_string(off.desktop_file.as_ref().unwrap()).unwrap();
    assert!(
        !desktop.contains("bwrap"),
        "sandbox-off launcher must not gain a wrapper:\n{desktop}"
    );
    assert!(desktop.contains(&off.target_bin.display().to_string()));
}

// ---------------------------------------------------------------------------
// AppImage-only scope and $PATH wrapper regeneration
// ---------------------------------------------------------------------------

/// Staleness detection and `--fix` regeneration only apply to AppImages: a
/// script (or extracted binary) whose baked `Exec` no longer matches the
/// policy is never reported and never regenerated.
#[test]
fn test_doctor_ignores_non_appimage_targets_even_when_exec_is_stale() {
    let temp = tempdir().unwrap();
    let base = temp.path();
    let manager = manager_with(base);

    write_settings(base, &bwrap_settings());
    // Registered with an explicit `--sandbox` (the only way a non-AppImage
    // gets a wrapper), so both launchers go stale after the edit.
    add_script_app(
        &manager,
        base,
        "Script App",
        AddOptions {
            sandbox: Some(true),
            ..AddOptions::default()
        },
    );
    add_app(&manager, base, "Stale App", AddOptions::default());

    let mut changed = bwrap_settings();
    changed.sandbox.policy.network = Some(false);
    write_settings(base, &changed);

    let report = run_doctor(&manager, false, false).unwrap();
    let stale = stale_details(&report);
    assert_eq!(
        stale.len(),
        1,
        "only the AppImage launcher may be reported: {:?}",
        report.details
    );
    assert!(stale[0].contains("stale-app"));
    assert!(
        !report.details.iter().any(|d| d.contains("script-app")),
        "a non-AppImage target must never be reported as out of date: {:?}",
        report.details
    );

    // --fix must leave the non-AppImage launcher exactly as it was.
    let script = manager.get("script-app").unwrap();
    let desktop_path = script.desktop_file.clone().unwrap();
    let before = fs::read_to_string(&desktop_path).unwrap();
    assert!(
        before.contains("--share-net"),
        "precondition: registered under the network-allowed policy:\n{before}"
    );

    run_doctor(&manager, true, true).unwrap();

    let after = fs::read_to_string(&desktop_path).unwrap();
    assert_eq!(
        after, before,
        "--fix must never regenerate a non-AppImage launcher"
    );
}

/// For a stale AppImage, `--fix` regenerates the `$PATH` wrapper under the
/// bin directory along with the `.desktop` launcher: the wrapper script
/// embeds the new sandbox command and no longer contains the old one.
#[test]
fn test_doctor_fix_regenerates_appimage_path_wrapper() {
    let temp = tempdir().unwrap();
    let base = temp.path();
    let manager = manager_with(base);

    write_settings(base, &bwrap_settings());
    add_app(&manager, base, "Stale App", AddOptions::default());

    let entry = manager.get("stale-app").unwrap();
    let wrapper_path = entry
        .symlink
        .clone()
        .expect("sandboxed AppImage must get a $PATH wrapper at add time");
    let old = fs::read_to_string(&wrapper_path).unwrap();
    assert!(
        old.contains("--share-net"),
        "precondition: wrapper baked under the network-allowed policy:\n{old}"
    );

    // New global policy: no network.
    let mut changed = bwrap_settings();
    changed.sandbox.policy.network = Some(false);
    write_settings(base, &changed);

    run_doctor(&manager, true, true).unwrap();

    let wrapper = fs::read_to_string(&wrapper_path).unwrap();
    assert!(
        wrapper.starts_with("#!/bin/sh\nexec "),
        "the $PATH entry must stay a wrapper script:\n{wrapper}"
    );
    assert!(wrapper.contains("bwrap"), "got:\n{wrapper}");
    assert!(
        !wrapper.contains("--share-net"),
        "the wrapper must no longer contain the old command:\n{wrapper}"
    );
    assert!(
        wrapper.contains(&entry.target_bin.display().to_string()),
        "the wrapper must still exec the registered target:\n{wrapper}"
    );
    // The target binary is never touched (non-destructive invariant).
    assert!(entry.target_bin.exists());
}

/// `--fix` regenerates the wrapper only for the stale AppImage: a sibling
/// AppImage with the sandbox turned off keeps its plain symlink pointing at
/// the target.
#[test]
fn test_doctor_fix_preserves_sandbox_off_symlink() {
    let temp = tempdir().unwrap();
    let base = temp.path();
    let manager = manager_with(base);

    write_settings(base, &bwrap_settings());
    add_app(
        &manager,
        base,
        "SandboxOff App",
        AddOptions {
            sandbox: Some(false),
            ..Default::default()
        },
    );
    add_app(&manager, base, "Stale App", AddOptions::default());

    let off = manager.get("sandboxoff-app").unwrap();
    let off_link = off
        .symlink
        .clone()
        .expect("AppImage symlink_appimages is on in these tests");
    assert!(
        fs::symlink_metadata(&off_link)
            .unwrap()
            .file_type()
            .is_symlink(),
        "precondition: sandbox-off app gets a plain symlink"
    );

    let mut changed = bwrap_settings();
    changed.sandbox.policy.network = Some(false);
    write_settings(base, &changed);

    run_doctor(&manager, true, true).unwrap();

    // The stale sibling's wrapper WAS regenerated (the new code path ran
    // during this very --fix pass)...
    let stale = manager.get("stale-app").unwrap();
    let sibling_wrapper = fs::read_to_string(stale.symlink.as_ref().unwrap()).unwrap();
    assert!(
        !sibling_wrapper.contains("--share-net"),
        "sibling wrapper must be regenerated to the new policy:\n{sibling_wrapper}"
    );

    // ...while the sandbox-off app's symlink is left untouched.
    let meta = fs::symlink_metadata(&off_link).unwrap();
    assert!(
        meta.file_type().is_symlink(),
        "sandbox-off link must remain a symlink (not a wrapper script)"
    );
    assert_eq!(
        fs::read_link(&off_link).unwrap(),
        off.target_bin,
        "sandbox-off link must still point at the target"
    );
    let desktop = fs::read_to_string(off.desktop_file.as_ref().unwrap()).unwrap();
    assert!(
        !desktop.contains("bwrap"),
        "sandbox-off launcher must not gain a wrapper:\n{desktop}"
    );
    assert!(desktop.contains(&off.target_bin.display().to_string()));
}
