use crate::config;
use crate::desktop;
use crate::extractor;
use crate::manager::Manager;
use crate::sandbox;
use anyhow::Result;

pub struct DoctorReport {
    pub passed: usize,
    pub issues: usize,
    pub details: Vec<String>,
}

pub fn run_doctor(manager: &Manager, fix: bool, yes: bool) -> Result<DoctorReport> {
    let mut report = DoctorReport {
        passed: 0,
        issues: 0,
        details: Vec::new(),
    };

    let base_dir = manager.settings_base();
    let settings = config::load_settings(base_dir)?;
    let registry = config::load(manager.config_path())?;

    // DirectoryCheck
    let bin_dir = dirs::home_dir()
        .map(|h| h.join(".local/bin"))
        .unwrap_or_default();
    let data_dir = dirs::home_dir()
        .map(|h| h.join(".local/share/applications"))
        .unwrap_or_default();
    let icons_dir = dirs::home_dir()
        .map(|h| h.join(".local/share/icons"))
        .unwrap_or_default();
    let config_dir = base_dir.to_path_buf();

    for (name, dir) in [
        ("bin", &bin_dir),
        ("applications", &data_dir),
        ("icons", &icons_dir),
        ("config", &config_dir),
    ] {
        if dir.exists() && dir.is_dir() {
            report.passed += 1;
        } else {
            report.issues += 1;
            report
                .details
                .push(format!("✖ Directory missing: {} ({})", name, dir.display()));
        }
    }

    let mut to_remove = Vec::new();
    let mut to_update = Vec::new(); // (id, remove_symlink)
    let mut stale_launchers = Vec::new(); // (id) sandbox Exec differs from policy

    for app in &registry.apps {
        // TargetCheck
        if !app.target_bin.exists() {
            report.issues += 1;

            report.details.push(format!(
                "✖ '{}' ({}) target missing: {}",
                app.name,
                app.id,
                app.target_bin.display()
            ));
            to_remove.push(app.id.clone());
        } else {
            report.passed += 1;
        }

        // DesktopFileCheck
        if let Some(desktop) = &app.desktop_file {
            if !desktop.exists() {
                report.issues += 1;

                report.details.push(format!(
                    "✖ '{}' ({}) desktop file missing: {}",
                    app.name,
                    app.id,
                    desktop.display()
                ));
            } else {
                report.passed += 1;
            }
        }

        // SymlinkCheck
        if let Some(link) = &app.symlink {
            if !link.exists() {
                report.issues += 1;

                report.details.push(format!(
                    "✖ '{}' ({}) symlink missing/broken: {}",
                    app.name,
                    app.id,
                    link.display()
                ));
                // If target exists but symlink is broken, maybe we just prune the symlink
                if app.target_bin.exists() {
                    to_update.push(app.id.clone());
                }
            } else {
                report.passed += 1;
            }
        }

        // SandboxLauncherCheck: the sandbox wrapper is baked into the
        // `Exec=` line at registration time, so editing the global
        // `[sandbox.policy]` leaves existing launchers running the old
        // command. Only AppImages are covered — their wrapper also lives in
        // the `$PATH` link, and `--fix` regenerates both entry points; other
        // executables (scripts, extracted binaries) keep whatever wrapper
        // they have and need a manual `musu update <app>`. Apps without a
        // launcher (covered above) or with the sandbox off are never flagged.
        if let Some(desktop_file) = &app.desktop_file
            && desktop_file.exists()
            && sandbox::sandbox_effective_enabled(&settings, app.sandbox, &app.target_bin)
            && extractor::is_appimage(&app.target_bin).unwrap_or(false)
        {
            let expected = sandbox::generate_sandbox_exec(
                &settings,
                app.sandbox,
                app.sandbox_policy.as_ref(),
                &app.target_bin,
            );
            if desktop::exec_out_of_date(desktop_file, &expected) {
                report.issues += 1;
                report.details.push(format!(
                    "✖ sandbox launcher out of date: {} ({}) (run `musu doctor --fix` to regenerate)",
                    app.id, app.name
                ));
                stale_launchers.push(app.id.clone());
            } else {
                report.passed += 1;
            }
        }
    }

    if fix {
        let mut modified = false;
        // prune broken symlinks
        for id in &to_update {
            if !to_remove.contains(id)
                && (yes || confirm(&format!("Remove broken symlink for '{}'?", id)))
            {
                manager.update(
                    id,
                    crate::manager::UpdateOptions {
                        remove_symlink: true,
                        ..Default::default()
                    },
                )?;
                modified = true;
            }
        }
        // regenerate outdated sandbox launchers
        for id in &stale_launchers {
            if !to_remove.contains(id)
                && (yes || confirm(&format!("Regenerate sandbox launcher for '{}'?", id)))
            {
                manager.regenerate_launcher(id)?;
                modified = true;
            }
        }
        // clean up dead entries
        for id in &to_remove {
            if yes || confirm(&format!("Remove dead entry '{}'?", id)) {
                manager.remove(id)?;
                modified = true;
            }
        }
        if modified {
            report
                .details
                .push("✔ Auto-repair applied successfully.".to_string());
        }
    }

    Ok(report)
}

fn confirm(msg: &str) -> bool {
    use std::io::Write;
    eprint!("{} [y/N]: ", msg);
    let _ = std::io::stderr().flush();
    let mut answer = String::new();
    let _ = std::io::stdin().read_line(&mut answer);
    matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}
