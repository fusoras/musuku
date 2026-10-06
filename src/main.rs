use anyhow::Result;
use clap::{CommandFactory, Parser};
use geico::{
    categories,
    cli::{Cli, Command},
    colors, config, desktop,
    manager::{AddOptions, Manager, UpdateOptions, UpdateOutcome},
    symlink,
};

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
            manager.add(AddOptions {
                target: args.executable,
                name: args.name,
                icon: args.icon,
                categories: args.categories,
                bin_name: args.bin_name,
                no_desktop: args.no_desktop,
                no_symlink: args.no_symlink,
                dest_bin: args.dest_bin,
            })?;
        }
        Command::List(args) => print!(
            "{}",
            manager.list_with_options(geico::manager::ListOptions {
                image: args.image,
                no_image: args.no_image,
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
                    image: args.image,
                    no_image: args.no_image,
                },
            )?
        ),
        Command::Update(args) => {
            let outcome = manager.update(
                &args.id_or_name,
                UpdateOptions {
                    target: args.target,
                    icon: args.icon,
                    name: args.name,
                    categories: args.categories,
                },
            )?;
            match outcome {
                UpdateOutcome::NoChanges { name } => println!(
                    "{}No changes specified for '{}'. Use --target, --name, --icon, or --categories to update.{}",
                    colors::BOLD_YELLOW,
                    name,
                    colors::RESET
                ),
                UpdateOutcome::Updated { entry } => println!(
                    "{}Updated '{}' successfully.{}",
                    colors::BOLD_GREEN,
                    entry.name,
                    colors::RESET
                ),
            }
        }
        Command::Remove(args) => manager.remove(&args.id_or_name)?,
        Command::Categories => print!("{}", categories::format_categories_table()),
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
