#![forbid(unsafe_code)]

use anyhow::{Result, bail};
use clap::Parser;
use musu::{
    categories,
    cli::{Cli, Command},
    colors, config,
    config::SandboxPolicy,
    desktop,
    manager::{AddOptions, Manager, RemoveOptions, UpdateOptions, UpdateOutcome},
    symlink,
};
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    let _ = musu::completions::ensure_completions_installed();
    let cli = Cli::parse();
    let manager = Manager::new(
        config::default_config_path(),
        desktop::default_base_dir(),
        symlink::default_base_dir(),
    );
    match cli.command {
        Command::Add(args) => {
            if matches!(args.categories.as_deref(), Some([])) {
                // `--categories` without a value: list mode, nothing is registered.
                print!(
                    "{}",
                    categories::render_categories_query(
                        None,
                        "Usage tip: musu add <path> --categories Video,AudioVideo",
                    )
                );
                return Ok(());
            }
            warn_unknown_categories(args.categories.as_deref().unwrap_or_default());
            let (description, descriptions) =
                musu::manager::parse_description_values(args.description)?;
            let sandbox_policy = build_sandbox_policy(
                args.network,
                args.home_writable,
                &args.hide_path,
                &args.extra_rw,
                &args.extra_ro,
            );
            warn_sandbox_policy(&manager, Some(&args.executable), sandbox_policy.as_ref());
            manager.add(AddOptions {
                target: args.executable,
                name: args.name,
                icon: args.icon,
                categories: args.categories,
                description,
                descriptions,
                bin_name: args.bin_name,
                no_desktop: args.no_desktop,
                no_symlink: args.no_symlink,
                dest_bin: args.dest_bin,
                sandbox: if args.sandbox {
                    Some(true)
                } else if args.no_sandbox {
                    Some(false)
                } else {
                    None
                },
                sandbox_policy,
                overwrite: args.overwrite,
                portable: args.portable,
            })?;
        }
        Command::List(args) => print!(
            "{}",
            manager.list_with_options(musu::manager::ListOptions {
                sort: args.sort,
                icons: args.icons,
                no_icons: args.no_icons,
                image: args.icons,
                no_image: args.no_icons,
                table: args.table,
                category: args.category,
                broken: args.broken,
                hide_disabled_symlinks: args.hide_disabled_symlinks,
                hide_target_ok: args.hide_target_ok,
                hide_desktop_ok: args.hide_desktop_ok,
                hide_ok: args.hide_ok,
                description: args.description,
                no_description: args.no_description,
                ids: args.ids,
                json: args.json,
            })?
        ),
        Command::Show(args) => print!(
            "{}",
            manager.show(
                &args.id_or_name,
                musu::manager::ShowOptions {
                    desktop: args.desktop,
                    bin: args.bin,
                    icon: args.icon,
                    symlink: args.symlink,
                    json: args.json,
                    absolute: args.absolute,
                    icons: args.icons,
                    no_icons: args.no_icons,
                    image: args.icons,
                    no_image: args.no_icons,
                },
            )?
        ),
        Command::Update(args) => {
            if matches!(args.categories.as_deref(), Some([])) {
                // `--categories` without a value: query mode, never mutates.
                let entry = manager.get(&args.id_or_name)?;
                let tip = format!(
                    "Usage tip: musu update {} --categories Video,AudioVideo",
                    entry.name
                );
                print!(
                    "{}",
                    categories::render_categories_query(
                        Some((
                            entry.name.as_str(),
                            entry.categories.as_deref().unwrap_or_default(),
                        )),
                        &tip,
                    )
                );
                return Ok(());
            }
            warn_unknown_categories(args.categories.as_deref().unwrap_or_default());
            let (description, descriptions) =
                musu::manager::parse_description_values(args.description)?;
            let sandbox_policy = build_sandbox_policy(
                args.network,
                args.home_writable,
                &args.hide_path,
                &args.extra_rw,
                &args.extra_ro,
            );
            // Warn against the policy that will actually be in effect: the
            // CLI policy when given, otherwise the app's stored one. The
            // target (new or stored) selects the app type, and therefore
            // the profile whose backend enforces the policy.
            let stored = manager.get(&args.id_or_name).ok();
            let effective_app_policy = sandbox_policy.clone().or_else(|| {
                stored
                    .as_ref()
                    .and_then(|entry| entry.sandbox_policy.clone())
            });
            let target = args
                .target
                .clone()
                .or_else(|| stored.as_ref().map(|entry| entry.target_bin.clone()));
            warn_sandbox_policy(&manager, target.as_deref(), effective_app_policy.as_ref());
            let outcome = manager.update(
                &args.id_or_name,
                UpdateOptions {
                    target: args.target,
                    icon: args.icon,
                    name: args.name,
                    categories: args.categories,
                    description,
                    descriptions,
                    remove_symlink: args.remove_symlink,
                    sandbox: if args.sandbox {
                        Some(true)
                    } else if args.no_sandbox {
                        Some(false)
                    } else {
                        None
                    },
                    sandbox_policy,
                    portable: args.portable,
                },
            )?;
            match outcome {
                UpdateOutcome::NoChanges { name } => println!(
                    "{}No changes specified for '{}'. Use --target, --name, --icon, --categories, --description, or --remove-symlink to update.{}",
                    colors::BOLD_YELLOW,
                    name,
                    colors::RESET
                ),
                UpdateOutcome::Updated { old, new } => {
                    println!(
                        "{}Updated '{}' successfully.{}",
                        colors::BOLD_GREEN,
                        new.name,
                        colors::RESET
                    );
                    print!("{}", musu::manager::format_visual_diff(&old, &new));
                }
            }
        }
        Command::Remove(args) => {
            if args.all {
                let registry = config::load(&config::default_config_path())?;
                if registry.apps.is_empty() {
                    println!("No applications registered in musu.");
                    return Ok(());
                }
                if !args.yes {
                    if !io::stdin().is_terminal() {
                        bail!(
                            "removing all applications requires confirmation but stdin is not interactive; re-run with --yes"
                        );
                    }
                    let warning = if args.delete_target {
                        " This will also permanently delete their target executables from disk."
                    } else {
                        ""
                    };
                    eprint!(
                        "Are you sure you want to remove all {} registered applications?{} [y/N]: ",
                        registry.apps.len(),
                        warning
                    );
                    io::stderr().flush()?;
                    let mut answer = String::new();
                    io::stdin().read_line(&mut answer)?;
                    if !matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
                        println!("Aborted: no applications were removed.");
                        return Ok(());
                    }
                }
                let entries = registry.apps;
                for entry in entries {
                    manager.remove_with_options(
                        &entry.id,
                        RemoveOptions {
                            delete_target: args.delete_target,
                        },
                    )?;
                    let suffix = if args.delete_target {
                        " and deleted target executable"
                    } else {
                        ""
                    };
                    println!(
                        "{}Removed {} ({}){}{suffix}",
                        colors::BOLD_GREEN,
                        entry.name,
                        entry.id,
                        colors::RESET
                    );
                }
            } else {
                for id_or_name in args.id_or_names {
                    let entry = manager.get(&id_or_name)?;
                    if args.delete_target && !args.yes {
                        let path = entry.target_bin.display().to_string();
                        if !io::stdin().is_terminal() {
                            bail!(
                                "--delete-target requires confirmation but stdin is not interactive; re-run with --yes to permanently delete '{path}'"
                            );
                        }
                        eprint!("Are you sure you want to permanently delete '{path}'? [y/N]: ");
                        io::stderr().flush()?;
                        let mut answer = String::new();
                        io::stdin().read_line(&mut answer)?;
                        if !matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
                            println!("Aborted: executable was not deleted.");
                            return Ok(());
                        }
                    }
                    manager.remove_with_options(
                        &id_or_name,
                        RemoveOptions {
                            delete_target: args.delete_target,
                        },
                    )?;
                    let suffix = if args.delete_target {
                        " and deleted target executable"
                    } else {
                        ""
                    };
                    println!(
                        "{}Removed {} ({}){}{suffix}",
                        colors::BOLD_GREEN,
                        entry.name,
                        entry.id,
                        colors::RESET
                    );
                }
            }
        }
        Command::Health(args) => print!("{}", manager.health(args.json, args.verbose)?),
        Command::Doctor(args) => {
            let report = musu::doctor::run_doctor(&manager, args.fix, args.yes)?;
            println!("diagnostics:");
            for detail in report.details {
                println!("  {}", detail);
            }
            println!(
                "\nSummary: {} passed, {} issues found.",
                report.passed, report.issues
            );
        }
        Command::Run(args) => manager.run(&args.id_or_name, &args.args)?,
        Command::Watch(args) => musu::watcher::run(args, manager)?,
        Command::Completions(args) => {
            let shell = args.shell.unwrap_or_else(|| {
                musu::completions::detect_shell(std::env::var("SHELL").ok().as_deref())
            });
            if args.stdout {
                musu::completions::generate_completions(shell, &mut std::io::stdout());
            } else {
                let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
                let path = musu::completions::install_completions_to(shell, &home)?;
                println!(
                    "{}Installed shell completions for {:?} to {}{}",
                    colors::BOLD_GREEN,
                    shell,
                    path.display(),
                    colors::RESET
                );
                if shell == clap_complete::Shell::Zsh {
                    let zshrc = musu::completions::find_zshrc_path(&home);
                    let _ = musu::completions::ensure_zshrc_source_line(&zshrc);
                }
                if musu::completions::is_carapace_installed() {
                    let _ = musu::completions::install_carapace_spec(&home);
                }
            }
        }
    }
    Ok(())
}

/// Emits an advisory warning to stderr for every `--categories` value that
/// would be kept as an unknown custom category. Never fatal, never stdout
/// (keeps stdout script-friendly), and colored via [`colors::should_color`].
fn warn_unknown_categories(values: &[String]) {
    for value in values {
        if let Some(warning) = categories::unknown_category_warning(value, colors::should_color()) {
            eprintln!("{warning}");
        }
    }
}

/// Build the per-app sandbox policy from the CLI flags: `None` when no
/// policy flag was given (inherit the type profile's policy), otherwise a
/// policy where every given flag overrides the profile field — in either
/// direction, so a per-app `--network true` exempts the app from a profile
/// `network = false` — and the remaining fields stay `None` (inherit).
fn build_sandbox_policy(
    network: Option<bool>,
    home_writable: Option<bool>,
    hide_path: &[PathBuf],
    extra_rw: &[PathBuf],
    extra_ro: &[PathBuf],
) -> Option<SandboxPolicy> {
    let any_flag = network.is_some()
        || home_writable.is_some()
        || !hide_path.is_empty()
        || !extra_rw.is_empty()
        || !extra_ro.is_empty();
    any_flag.then(|| SandboxPolicy {
        network,
        home_writable,
        hide_paths: (!hide_path.is_empty()).then(|| paths_to_strings(hide_path)),
        extra_rw: (!extra_rw.is_empty()).then(|| paths_to_strings(extra_rw)),
        extra_ro: (!extra_ro.is_empty()).then(|| paths_to_strings(extra_ro)),
    })
}

fn paths_to_strings(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect()
}

/// Emit advisory sandbox-policy warnings for restrictions the resolved
/// backend cannot enforce, styled like [`warn_unknown_categories`]. Never
/// fatal, always stderr (stdout stays script-friendly). `target` selects
/// the app type whose profile applies; without it there is nothing to warn
/// about yet.
fn warn_sandbox_policy(
    manager: &Manager,
    target: Option<&Path>,
    app_policy: Option<&SandboxPolicy>,
) {
    let (Ok(settings), Some(target)) = (manager.load_settings(), target) else {
        return;
    };
    let kind = musu::sandbox::classify(target);
    let backend = musu::sandbox::resolve_backend(&settings, kind);
    let effective = musu::sandbox::effective_settings_policy(&settings, kind, app_policy);
    let color = colors::should_color();
    for warning in musu::sandbox::validate_policy(&backend, &effective) {
        eprintln!(
            "{}",
            colors::yellow(color, &format!("musu: warning: {warning}"))
        );
    }
}
