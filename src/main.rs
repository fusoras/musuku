#![forbid(unsafe_code)]

use anyhow::{Result, bail};
use clap::{CommandFactory, Parser};
use geico::{
    categories,
    cli::{Cli, Command},
    colors, config, desktop,
    manager::{AddOptions, Manager, RemoveOptions, UpdateOptions, UpdateOutcome},
    symlink,
};
use std::io::{self, IsTerminal, Write};

fn main() -> Result<()> {
    let _ = geico::completions::ensure_completions_installed();
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
                        "Usage tip: geico add <path> --categories Video,AudioVideo",
                    )
                );
                return Ok(());
            }
            warn_unknown_categories(args.categories.as_deref().unwrap_or_default());
            manager.add(AddOptions {
                target: args.executable,
                name: args.name,
                icon: args.icon,
                categories: args.categories,
                description: args.description,
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
            })?;
        }
        Command::List(args) => print!(
            "{}",
            manager.list_with_options(geico::manager::ListOptions {
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
                geico::manager::ShowOptions {
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
                    "Usage tip: geico update {} --categories Video,AudioVideo",
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
            let outcome = manager.update(
                &args.id_or_name,
                UpdateOptions {
                    target: args.target,
                    icon: args.icon,
                    name: args.name,
                    categories: args.categories,
                    description: args.description,
                    remove_symlink: args.remove_symlink,
                    sandbox: if args.sandbox {
                        Some(true)
                    } else if args.no_sandbox {
                        Some(false)
                    } else {
                        None
                    },
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
                    print!("{}", geico::manager::format_visual_diff(&old, &new));
                }
            }
        }
        Command::Remove(args) => {
            if args.all {
                let registry = config::load(&config::default_config_path())?;
                if registry.apps.is_empty() {
                    println!("No applications registered in geico.");
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
            let report = geico::doctor::run_doctor(&manager, args.fix, args.yes)?;
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
        Command::Watch(args) => {
            if let Some(service_cmd) = args.service {
                geico::service::dispatch(service_cmd, &manager)?;
            } else {
                geico::watcher::run(args, manager)?;
            }
        }
        Command::Completions(args) => {
            let shell = args.shell.unwrap_or_else(|| {
                geico::completions::detect_shell(std::env::var("SHELL").ok().as_deref())
            });
            if args.stdout {
                clap_complete::generate(
                    shell,
                    &mut Cli::command(),
                    "geico",
                    &mut std::io::stdout(),
                );
            } else {
                let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
                let path = geico::completions::install_completions_to(shell, &home)?;
                println!(
                    "{}Installed shell completions for {:?} to {}{}",
                    colors::BOLD_GREEN,
                    shell,
                    path.display(),
                    colors::RESET
                );
                if shell == clap_complete::Shell::Zsh {
                    let zshrc = geico::completions::find_zshrc_path(&home);
                    let _ = geico::completions::ensure_zshrc_source_line(&zshrc);
                }
                if geico::completions::is_carapace_installed() {
                    let _ = geico::completions::install_carapace_spec(&home);
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
