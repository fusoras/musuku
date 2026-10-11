//! Sandbox policy: merge semantics, per-backend command generation,
//! validation warnings, serde roundtrips, and `musu show` output.

use musu::config::{self, SandboxPolicy, Settings};
use musu::manager::{AddOptions, Manager, UpdateOptions, UpdateOutcome};
use musu::models::{AppEntry, AppRegistry};
use musu::sandbox::{effective_settings_policy, generate_sandbox_exec, validate_policy};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn bwrap_settings() -> Settings {
    let mut settings = Settings::default();
    settings.sandbox.enabled = true;
    settings.sandbox.backend = "bwrap".to_string();
    settings
}

fn executable(path: &Path) {
    fs::write(path, "#!/bin/sh\necho hi\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

/// Fake AppImage target (repo-wide test pattern): `extractor::is_appimage`
/// accepts a non-empty file with a case-insensitive `.AppImage` extension,
/// and the shebang body keeps the `auto_extract` probe harmless.
fn appimage(path: &Path) {
    executable(path);
}

// ---------------------------------------------------------------------------
// effective_policy: field-by-field merge
// ---------------------------------------------------------------------------

#[test]
fn test_effective_policy_merges_field_by_field() {
    let global = SandboxPolicy {
        network: Some(true),
        home_writable: Some(true),
        hide_paths: Some(vec!["/global-hidden".to_string()]),
        extra_rw: None,
        extra_ro: None,
    };
    let app = SandboxPolicy {
        network: Some(false),
        home_writable: None,
        hide_paths: None,
        extra_rw: Some(vec!["/rw".to_string()]),
        extra_ro: None,
    };
    let merged = config::effective_policy(&global, Some(&app));
    // `Some` in the app replaces the global value.
    assert_eq!(merged.network, Some(false));
    assert_eq!(
        merged.extra_rw.as_deref(),
        Some(&["/rw".to_string()][..]),
        "app list replaces the global list"
    );
    // `None` in the app inherits the global value.
    assert_eq!(merged.home_writable, Some(true));
    assert_eq!(
        merged.hide_paths.as_deref(),
        Some(&["/global-hidden".to_string()][..])
    );
    // Unset everywhere falls back to the effective defaults.
    assert_eq!(merged.extra_ro, Some(Vec::new()));
}

#[test]
fn test_effective_policy_defaults_when_nothing_defined() {
    let merged = config::effective_policy(&SandboxPolicy::default(), None);
    assert_eq!(merged.network, Some(true));
    assert_eq!(merged.home_writable, Some(true));
    assert_eq!(merged.hide_paths, Some(Vec::new()));
    assert_eq!(merged.extra_rw, Some(Vec::new()));
    assert_eq!(merged.extra_ro, Some(Vec::new()));
}

#[test]
fn test_global_effective_fills_defaults() {
    let settings = Settings::default();
    let policy = settings.sandbox.global_effective();
    assert_eq!(policy.network, Some(true));
    assert_eq!(policy.home_writable, Some(true));
    assert_eq!(policy.hide_paths, Some(Vec::new()));
    assert_eq!(policy.extra_rw, Some(Vec::new()));
    assert_eq!(policy.extra_ro, Some(Vec::new()));

    // An explicit `false` in the global config survives the defaults pass.
    let settings: Settings =
        toml::from_str("[sandbox]\nenabled = true\n[sandbox.policy]\nnetwork = false\n").unwrap();
    let policy = settings.sandbox.global_effective();
    assert_eq!(policy.network, Some(false));
    assert_eq!(policy.home_writable, Some(true));
}

#[test]
fn test_effective_settings_policy_merges_global_and_app() {
    let mut settings = bwrap_settings();
    settings.sandbox.policy.hide_paths = Some(vec!["/global-hidden".to_string()]);
    let app = SandboxPolicy {
        network: Some(false),
        ..Default::default()
    };
    let policy = effective_settings_policy(&settings, Some(&app));
    assert_eq!(policy.network, Some(false));
    assert_eq!(
        policy.hide_paths.as_deref(),
        Some(&["/global-hidden".to_string()][..])
    );
    assert_eq!(policy.home_writable, Some(true));
}

// ---------------------------------------------------------------------------
// generate_sandbox_exec: bwrap policy flags
// ---------------------------------------------------------------------------

#[test]
fn test_bwrap_network_false_omits_share_net() {
    let settings = bwrap_settings();
    let policy = SandboxPolicy {
        network: Some(false),
        ..Default::default()
    };
    let cmd = generate_sandbox_exec(&settings, None, Some(&policy), Path::new("/tmp/bin"));
    assert!(cmd.starts_with("bwrap "), "got: {cmd}");
    assert!(cmd.contains("--unshare-all"), "got: {cmd}");
    assert!(!cmd.contains("--share-net"), "got: {cmd}");
}

#[test]
fn test_bwrap_home_writable_false_ro_binds_home() {
    let settings = bwrap_settings();
    let home = dirs::home_dir().unwrap();
    let home_str = home.display().to_string();
    let policy = SandboxPolicy {
        home_writable: Some(false),
        ..Default::default()
    };
    let cmd = generate_sandbox_exec(&settings, None, Some(&policy), Path::new("/tmp/bin"));
    assert!(
        cmd.contains(&format!("--ro-bind {home_str} {home_str}")),
        "got: {cmd}"
    );
    assert!(
        !cmd.contains(&format!("--bind {home_str} {home_str}")),
        "got: {cmd}"
    );
}

#[test]
fn test_bwrap_hide_paths_extra_rw_extra_ro_emit_flags() {
    let settings = bwrap_settings();
    let policy = SandboxPolicy {
        hide_paths: Some(vec!["/data/secret".to_string()]),
        extra_rw: Some(vec!["/data/rw".to_string()]),
        extra_ro: Some(vec!["/data/ro".to_string()]),
        ..Default::default()
    };
    let cmd = generate_sandbox_exec(&settings, None, Some(&policy), Path::new("/tmp/bin"));
    assert!(cmd.contains("--tmpfs /data/secret"), "got: {cmd}");
    assert!(cmd.contains("--bind /data/rw /data/rw"), "got: {cmd}");
    assert!(cmd.contains("--ro-bind /data/ro /data/ro"), "got: {cmd}");
}

#[test]
fn test_bwrap_policy_paths_expand_tilde() {
    let settings = bwrap_settings();
    let home = dirs::home_dir().unwrap().display().to_string();
    let policy = SandboxPolicy {
        hide_paths: Some(vec!["~/secret".to_string()]),
        extra_rw: Some(vec!["~/scratch".to_string()]),
        extra_ro: Some(vec!["~/ro".to_string()]),
        ..Default::default()
    };
    let cmd = generate_sandbox_exec(&settings, None, Some(&policy), Path::new("/tmp/bin"));
    assert!(
        cmd.contains(&format!("--tmpfs {home}/secret")),
        "got: {cmd}"
    );
    assert!(
        cmd.contains(&format!("--bind {home}/scratch {home}/scratch")),
        "got: {cmd}"
    );
    assert!(
        cmd.contains(&format!("--ro-bind {home}/ro {home}/ro")),
        "got: {cmd}"
    );
}

// ---------------------------------------------------------------------------
// ~/.ssh and ~/.gnupg are always hidden
// ---------------------------------------------------------------------------

#[test]
fn test_bwrap_ssh_and_gnupg_always_hidden_even_if_policy_exposes_them() {
    let settings = bwrap_settings();
    let home = dirs::home_dir().unwrap().display().to_string();
    let policy = SandboxPolicy {
        hide_paths: Some(vec![format!("{home}/.ssh"), "~/.gnupg".to_string()]),
        extra_rw: Some(vec![format!("{home}/.ssh"), "~/.gnupg".to_string()]),
        extra_ro: Some(vec![format!("{home}/.gnupg")]),
        ..Default::default()
    };
    let cmd = generate_sandbox_exec(&settings, None, Some(&policy), Path::new("/tmp/bin"));
    assert!(
        cmd.matches(&format!("--tmpfs {home}/.ssh")).count() == 1,
        "exactly one tmpfs over ~/.ssh, got: {cmd}"
    );
    assert!(
        cmd.matches(&format!("--tmpfs {home}/.gnupg")).count() == 1,
        "exactly one tmpfs over ~/.gnupg, got: {cmd}"
    );
    assert!(
        !cmd.contains(&format!("--bind {home}/.ssh")),
        "~/.ssh must never be exposed read-write, got: {cmd}"
    );
    assert!(
        !cmd.contains(&format!("--bind {home}/.gnupg")),
        "~/.gnupg must never be exposed read-write, got: {cmd}"
    );
    assert!(
        !cmd.contains(&format!("--ro-bind {home}/.ssh")),
        "~/.ssh must never be exposed, got: {cmd}"
    );
    assert!(
        !cmd.contains(&format!("--ro-bind {home}/.gnupg")),
        "~/.gnupg must never be exposed, got: {cmd}"
    );
}

// ---------------------------------------------------------------------------
// Byte-for-byte compatibility when no policy is defined
// ---------------------------------------------------------------------------

#[test]
fn test_default_generation_is_byte_identical_to_previous_behavior() {
    let home = dirs::home_dir().unwrap().display().to_string();
    let target = "/tmp/bin";

    let mut settings = bwrap_settings();
    let expected_bwrap = format!(
        "bwrap --unshare-all --share-net --ro-bind / / --dev /dev --proc /proc --bind /tmp /tmp \
         --bind {home} {home} --tmpfs {home}/.ssh --tmpfs {home}/.gnupg {target}"
    );
    assert_eq!(
        generate_sandbox_exec(&settings, None, None, Path::new(target)),
        expected_bwrap
    );
    // An all-None per-app policy must produce the very same command.
    let policy = SandboxPolicy::default();
    assert_eq!(
        generate_sandbox_exec(&settings, None, Some(&policy), Path::new(target)),
        expected_bwrap
    );

    settings.sandbox.backend = "firejail".to_string();
    let expected_firejail = format!(
        "firejail --noprofile --net=host --env=WAYLAND_DISPLAY --env=DISPLAY \
         --env=XDG_RUNTIME_DIR --env=PULSE_SERVER --env=PIPEWIRE_REMOTE \
         --blacklist=~/.ssh --blacklist=~/.gnupg {target}"
    );
    assert_eq!(
        generate_sandbox_exec(&settings, None, None, Path::new(target)),
        expected_firejail
    );
    assert_eq!(
        generate_sandbox_exec(&settings, None, Some(&policy), Path::new(target)),
        expected_firejail
    );

    // Disabled sandbox still returns the plain target.
    settings.sandbox.enabled = false;
    assert_eq!(
        generate_sandbox_exec(&settings, None, None, Path::new(target)),
        target
    );
}

// ---------------------------------------------------------------------------
// firejail policy flags
// ---------------------------------------------------------------------------

#[test]
fn test_firejail_network_false_uses_net_none() {
    let mut settings = bwrap_settings();
    settings.sandbox.backend = "firejail".to_string();
    let policy = SandboxPolicy {
        network: Some(false),
        ..Default::default()
    };
    let cmd = generate_sandbox_exec(&settings, None, Some(&policy), Path::new("/tmp/bin"));
    assert!(cmd.contains("--net=none"), "got: {cmd}");
    assert!(!cmd.contains("--net=host"), "got: {cmd}");
}

#[test]
fn test_firejail_hide_paths_and_extra_ro_blacklist() {
    let mut settings = bwrap_settings();
    settings.sandbox.backend = "firejail".to_string();
    let home = dirs::home_dir().unwrap().display().to_string();
    let policy = SandboxPolicy {
        hide_paths: Some(vec!["~/secret".to_string(), format!("{home}/.ssh")]),
        extra_ro: Some(vec!["/data/ro".to_string()]),
        ..Default::default()
    };
    let cmd = generate_sandbox_exec(&settings, None, Some(&policy), Path::new("/tmp/bin"));
    assert!(
        cmd.contains(&format!("--blacklist={home}/secret")),
        "got: {cmd}"
    );
    assert!(cmd.contains("--blacklist=/data/ro"), "got: {cmd}");
    // ~/.ssh stays hidden (exactly once) and the fixed flags are preserved.
    assert_eq!(
        cmd.matches("--blacklist=").count(),
        4,
        "expected ssh + gnupg + secret + ro minus the ~/.ssh duplicate, got: {cmd}"
    );
    assert!(cmd.contains("--blacklist=~/.ssh"), "got: {cmd}");
    assert!(cmd.contains("--blacklist=~/.gnupg"), "got: {cmd}");
}

#[test]
fn test_firejail_ignores_home_writable_and_extra_rw() {
    let mut settings = bwrap_settings();
    settings.sandbox.backend = "firejail".to_string();
    let policy = SandboxPolicy {
        home_writable: Some(false),
        extra_rw: Some(vec!["/data/rw".to_string()]),
        ..Default::default()
    };
    let cmd = generate_sandbox_exec(&settings, None, Some(&policy), Path::new("/tmp/bin"));
    assert!(
        !cmd.contains("/data/rw"),
        "firejail cannot grant extra_rw; got: {cmd}"
    );
}

// ---------------------------------------------------------------------------
// validate_policy
// ---------------------------------------------------------------------------

#[test]
fn test_validate_policy_firejail_warns_for_unsupported_restrictions() {
    let policy = SandboxPolicy {
        home_writable: Some(false),
        extra_rw: Some(vec!["/data/rw".to_string()]),
        ..Default::default()
    };
    let warnings = validate_policy("firejail", &policy);
    assert_eq!(warnings.len(), 2, "got: {warnings:?}");
    assert!(
        warnings[0].contains("firejail") && warnings[0].contains("home_writable=false"),
        "got: {:?}",
        warnings[0]
    );
    assert!(warnings[1].contains("extra_rw"), "got: {:?}", warnings[1]);
}

#[test]
fn test_validate_policy_bwrap_is_silent() {
    let policy = SandboxPolicy {
        home_writable: Some(false),
        extra_rw: Some(vec!["/data/rw".to_string()]),
        network: Some(false),
        ..Default::default()
    };
    assert!(validate_policy("bwrap", &policy).is_empty());
    // Default firejail policy has nothing unsupported to warn about.
    assert!(validate_policy("firejail", &SandboxPolicy::default()).is_empty());
}

// ---------------------------------------------------------------------------
// AppEntry serde roundtrip
// ---------------------------------------------------------------------------

#[test]
fn test_app_entry_sandbox_policy_roundtrips_in_apps_toml() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("apps.toml");

    let mut registry = AppRegistry::new();
    let mut entry = AppEntry::new(Some("Foo"), PathBuf::from("/tmp/foo"));
    entry.sandbox_policy = Some(SandboxPolicy {
        network: Some(false),
        hide_paths: Some(vec!["~/secret".to_string()]),
        ..Default::default()
    });
    registry.add(entry.clone());
    config::save(&path, &registry).unwrap();

    let loaded = config::load(&path).unwrap();
    assert_eq!(loaded.apps.len(), 1);
    assert_eq!(loaded.apps[0].sandbox_policy, entry.sandbox_policy);

    // Entries without a policy must not serialize the key at all.
    let plain_path = temp.path().join("plain.toml");
    let mut plain = AppRegistry::new();
    plain.add(AppEntry::new(Some("Bar"), PathBuf::from("/tmp/bar")));
    config::save(&plain_path, &plain).unwrap();
    let text = fs::read_to_string(&plain_path).unwrap();
    assert!(!text.contains("sandbox_policy"), "got: {text}");
}

// ---------------------------------------------------------------------------
// Per-app policy EXEMPTS (bidirectional override) of the global policy
// ---------------------------------------------------------------------------

#[test]
fn test_app_network_true_exempts_global_network_false() {
    let mut settings = bwrap_settings();
    settings.sandbox.policy.network = Some(false);
    let app = SandboxPolicy {
        network: Some(true),
        ..Default::default()
    };
    let cmd = generate_sandbox_exec(&settings, None, Some(&app), Path::new("/tmp/bin"));
    assert!(
        cmd.contains("--share-net"),
        "app network=true must exempt the global network=false, got: {cmd}"
    );
}

#[test]
fn test_app_network_false_overrides_global_default() {
    let settings = bwrap_settings();
    let app = SandboxPolicy {
        network: Some(false),
        ..Default::default()
    };
    let cmd = generate_sandbox_exec(&settings, None, Some(&app), Path::new("/tmp/bin"));
    assert!(!cmd.contains("--share-net"), "got: {cmd}");
}

#[test]
fn test_app_home_writable_true_exempts_global_home_writable_false() {
    let mut settings = bwrap_settings();
    settings.sandbox.policy.home_writable = Some(false);
    let home = dirs::home_dir().unwrap().display().to_string();
    let app = SandboxPolicy {
        home_writable: Some(true),
        ..Default::default()
    };
    let cmd = generate_sandbox_exec(&settings, None, Some(&app), Path::new("/tmp/bin"));
    assert!(
        cmd.contains(&format!("--bind {home} {home}")),
        "app home_writable=true must exempt the global restriction, got: {cmd}"
    );
    assert!(
        !cmd.contains(&format!("--ro-bind {home} {home}")),
        "got: {cmd}"
    );
}

#[test]
fn test_app_home_writable_false_overrides_global_default() {
    let settings = bwrap_settings();
    let home = dirs::home_dir().unwrap().display().to_string();
    let app = SandboxPolicy {
        home_writable: Some(false),
        ..Default::default()
    };
    let cmd = generate_sandbox_exec(&settings, None, Some(&app), Path::new("/tmp/bin"));
    assert!(
        cmd.contains(&format!("--ro-bind {home} {home}")),
        "got: {cmd}"
    );
    assert!(
        !cmd.contains(&format!("--bind {home} {home}")),
        "got: {cmd}"
    );
}

// ---------------------------------------------------------------------------
// CLI parsing of the bidirectional policy flags
// ---------------------------------------------------------------------------

#[test]
fn test_cli_parses_network_and_home_writable_values_on_add_and_update() {
    use clap::Parser;
    use musu::cli::{Cli, Command};

    let cli = Cli::try_parse_from([
        "musu",
        "add",
        "./app",
        "--sandbox",
        "--network",
        "true",
        "--home-writable",
        "false",
    ])
    .expect("--network/--home-writable must accept explicit values");
    let Command::Add(args) = cli.command else {
        panic!("expected add");
    };
    assert_eq!(args.network, Some(true));
    assert_eq!(args.home_writable, Some(false));

    let cli = Cli::try_parse_from([
        "musu",
        "update",
        "my-app",
        "--network",
        "false",
        "--extra-ro",
        "/data",
    ])
    .expect("update must accept the policy flags too");
    let Command::Update(args) = cli.command else {
        panic!("expected update");
    };
    assert_eq!(args.network, Some(false));
    assert_eq!(args.home_writable, None);
    assert_eq!(args.extra_ro, vec![PathBuf::from("/data")]);

    // Omitting the flags keeps everything None (inherit the global policy).
    let cli = Cli::try_parse_from(["musu", "add", "./app"]).unwrap();
    let Command::Add(args) = cli.command else {
        panic!("expected add");
    };
    assert_eq!(args.network, None);
    assert_eq!(args.home_writable, None);
    assert!(args.hide_path.is_empty());
    assert!(args.extra_rw.is_empty());
    assert!(args.extra_ro.is_empty());
}

// ---------------------------------------------------------------------------
// manager: update with sandbox_policy regenerates the .desktop
// ---------------------------------------------------------------------------

#[test]
fn test_update_with_sandbox_policy_changes_entry_and_regenerates_desktop() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("tool.AppImage");
    appimage(&target);

    fs::create_dir_all(temp.path().join("config")).unwrap();
    fs::write(
        temp.path().join("config/config.toml"),
        "[sandbox]\nenabled = true\nbackend = \"bwrap\"\n",
    )
    .unwrap();

    let manager = Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );
    let entry = manager
        .add(AddOptions {
            target,
            ..AddOptions::default()
        })
        .unwrap();
    let desktop_before = fs::read_to_string(entry.desktop_file.as_ref().unwrap()).unwrap();
    assert!(
        desktop_before.contains("--share-net"),
        "got: {desktop_before}"
    );

    let policy = SandboxPolicy {
        network: Some(false),
        ..Default::default()
    };
    let outcome = manager
        .update(
            &entry.id,
            UpdateOptions {
                sandbox_policy: Some(policy.clone()),
                ..UpdateOptions::default()
            },
        )
        .unwrap();
    let UpdateOutcome::Updated { new, .. } = outcome else {
        panic!("expected the sandbox policy change to count as an update");
    };
    assert_eq!(new.sandbox_policy, Some(policy));

    let desktop_after = fs::read_to_string(new.desktop_file.as_ref().unwrap()).unwrap();
    assert!(desktop_after.contains("bwrap"), "got: {desktop_after}");
    assert!(
        !desktop_after.contains("--share-net"),
        "policy network=false must reach the regenerated .desktop, got: {desktop_after}"
    );
}

// ---------------------------------------------------------------------------
// musu show: sandbox section
// ---------------------------------------------------------------------------

#[test]
fn test_show_displays_sandbox_denied_permissions() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("tool.AppImage");
    appimage(&target);

    fs::create_dir_all(temp.path().join("config")).unwrap();
    fs::write(
        temp.path().join("config/config.toml"),
        "[sandbox]\nenabled = true\nbackend = \"bwrap\"\n[sandbox.policy]\nnetwork = false\n",
    )
    .unwrap();

    let manager = Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );
    let entry = manager
        .add(AddOptions {
            target,
            ..AddOptions::default()
        })
        .unwrap();

    let output = manager.show(&entry.id, Default::default()).unwrap();
    assert!(output.contains("Sandbox: active (bwrap)"), "got: {output}");
    assert!(output.contains("no network access"), "got: {output}");
    assert!(output.contains("~/.ssh"), "got: {output}");
    assert!(output.contains("~/.gnupg"), "got: {output}");

    let json = manager
        .show(
            &entry.id,
            musu::manager::ShowOptions {
                json: true,
                ..Default::default()
            },
        )
        .unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["sandbox"]["enabled"], true);
    assert_eq!(value["sandbox"]["backend"], "bwrap");
    assert_eq!(value["sandbox"]["policy"]["network"], false);
    let denied = value["sandbox"]["denied"].as_array().unwrap();
    assert!(
        denied
            .iter()
            .any(|d| d.as_str().unwrap().contains("~/.ssh")),
        "got: {denied:?}"
    );
    assert!(
        denied
            .iter()
            .any(|d| d.as_str().unwrap().contains("network")),
        "got: {denied:?}"
    );
}

#[test]
fn test_show_reports_sandbox_inactive() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("tool");
    executable(&target);
    let manager = Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );
    let entry = manager
        .add(AddOptions {
            target,
            ..AddOptions::default()
        })
        .unwrap();

    let output = manager.show(&entry.id, Default::default()).unwrap();
    assert!(output.contains("Sandbox: inactive"), "got: {output}");

    let json = manager
        .show(
            &entry.id,
            musu::manager::ShowOptions {
                json: true,
                ..Default::default()
            },
        )
        .unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["sandbox"]["enabled"], false);
    assert!(value["sandbox"]["denied"].as_array().unwrap().is_empty());
}

// ---------------------------------------------------------------------------
// add --sandbox persists the per-entry flag (regression: manager.add used to
// drop options.sandbox, so the launcher was never wrapped)
// ---------------------------------------------------------------------------

#[test]
fn test_add_with_sandbox_option_persists_flag_and_wraps_desktop() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("tool");
    executable(&target);

    // Global sandbox OFF: only the per-entry flag can enable the wrapper.
    fs::create_dir_all(temp.path().join("config")).unwrap();
    fs::write(
        temp.path().join("config/config.toml"),
        "[sandbox]\nenabled = false\nbackend = \"bwrap\"\n",
    )
    .unwrap();

    let manager = Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );
    let entry = manager
        .add(AddOptions {
            target,
            sandbox: Some(true),
            ..AddOptions::default()
        })
        .unwrap();

    assert_eq!(
        entry.sandbox,
        Some(true),
        "add must persist options.sandbox in the registry entry"
    );
    let registry = config::load(&temp.path().join("config/apps.toml")).unwrap();
    let stored = registry.find(&entry.id).expect("entry stored in apps.toml");
    assert_eq!(
        stored.sandbox,
        Some(true),
        "flag must survive the TOML roundtrip"
    );

    let desktop = fs::read_to_string(entry.desktop_file.as_ref().unwrap()).unwrap();
    assert!(
        desktop.contains("bwrap"),
        "desktop Exec must be wrapped in bwrap, got: {desktop}"
    );
}

#[test]
fn test_add_with_no_sandbox_option_stays_unwrapped() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("tool");
    executable(&target);

    // Global sandbox ON: add without flags must NOT force-sandbox the app.
    fs::create_dir_all(temp.path().join("config")).unwrap();
    fs::write(
        temp.path().join("config/config.toml"),
        "[sandbox]\nenabled = true\nbackend = \"bwrap\"\n",
    )
    .unwrap();

    let manager = Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );
    let entry = manager
        .add(AddOptions {
            target: target.clone(),
            ..AddOptions::default()
        })
        .unwrap();

    assert_eq!(entry.sandbox, None, "no flag means no explicit override");
    let desktop = fs::read_to_string(entry.desktop_file.as_ref().unwrap()).unwrap();
    // The global enabled=true flag only wraps AppImages; a script must stay
    // unwrapped even with the global sandbox on. The entry flag itself must
    // stay None in the registry.
    assert!(
        !desktop.contains("bwrap"),
        "a non-AppImage must not be wrapped by the global flag, got: {desktop}"
    );
    assert!(
        desktop.contains(&target.display().to_string()),
        "Exec must point straight at the target, got: {desktop}"
    );
    let link = entry
        .symlink
        .clone()
        .expect("non-AppImage targets are always symlinked");
    assert!(
        fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink(),
        "the $PATH entry must stay a plain symlink for a non-AppImage"
    );
    let registry = config::load(&temp.path().join("config/apps.toml")).unwrap();
    let stored = registry.find(&entry.id).unwrap();
    assert_eq!(stored.sandbox, None);
}

// ---------------------------------------------------------------------------
// Global sandbox enablement only covers AppImages; the explicit flag wins
// ---------------------------------------------------------------------------

/// The explicit `--sandbox` flag always wins: a script force-sandboxed by
/// the user gets the wrapper even though it is not an AppImage.
#[test]
fn test_add_script_with_explicit_sandbox_flag_gets_wrapper() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("tool");
    executable(&target);

    fs::create_dir_all(temp.path().join("config")).unwrap();
    fs::write(
        temp.path().join("config/config.toml"),
        "[sandbox]\nenabled = true\nbackend = \"bwrap\"\n",
    )
    .unwrap();

    let manager = Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );
    let entry = manager
        .add(AddOptions {
            target,
            sandbox: Some(true),
            ..AddOptions::default()
        })
        .unwrap();

    assert_eq!(entry.sandbox, Some(true));
    let desktop = fs::read_to_string(entry.desktop_file.as_ref().unwrap()).unwrap();
    assert!(
        desktop.contains("bwrap"),
        "an explicit --sandbox must wrap any executable, got: {desktop}"
    );
    let link = entry.symlink.clone().unwrap();
    assert!(
        fs::read_to_string(&link).unwrap().contains("bwrap"),
        "the $PATH wrapper must be generated too"
    );
}

/// No regression: an AppImage registered under a globally enabled sandbox
/// keeps getting the wrapper without any explicit flag.
#[test]
fn test_add_appimage_with_global_sandbox_enabled_gets_wrapper() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("tool.AppImage");
    appimage(&target);

    fs::create_dir_all(temp.path().join("config")).unwrap();
    fs::write(
        temp.path().join("config/config.toml"),
        "[sandbox]\nenabled = true\nbackend = \"bwrap\"\n",
    )
    .unwrap();

    let manager = Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );
    let entry = manager
        .add(AddOptions {
            target,
            ..AddOptions::default()
        })
        .unwrap();

    assert_eq!(entry.sandbox, None, "no flag means no explicit override");
    let desktop = fs::read_to_string(entry.desktop_file.as_ref().unwrap()).unwrap();
    assert!(
        desktop.contains("bwrap"),
        "a globally sandboxed AppImage must be wrapped, got: {desktop}"
    );
}

/// `musu show` reports the effective state: a script registered under a
/// globally enabled sandbox is inactive, in both the text and JSON views.
#[test]
fn test_show_reports_sandbox_inactive_for_script_with_global_enabled() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("tool");
    executable(&target);

    fs::create_dir_all(temp.path().join("config")).unwrap();
    fs::write(
        temp.path().join("config/config.toml"),
        "[sandbox]\nenabled = true\nbackend = \"bwrap\"\n",
    )
    .unwrap();

    let manager = Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );
    let entry = manager
        .add(AddOptions {
            target,
            ..AddOptions::default()
        })
        .unwrap();

    let output = manager.show(&entry.id, Default::default()).unwrap();
    assert!(
        output.contains("Sandbox: inactive"),
        "a non-AppImage without an explicit flag must report inactive, got: {output}"
    );

    let json = manager
        .show(
            &entry.id,
            musu::manager::ShowOptions {
                json: true,
                ..Default::default()
            },
        )
        .unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["sandbox"]["enabled"], false);
    assert!(value["sandbox"]["denied"].as_array().unwrap().is_empty());
}
