use crate::config;
use crate::manager::Manager;
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
    let _settings = config::load_settings(base_dir)?;
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
