use anyhow::Result;
use clap::Parser;
use geico::{
    cli::{Cli, Command},
    config, desktop,
    manager::{AddOptions, Manager, UpdateOptions, UpdateOutcome},
    symlink,
};

fn main() -> Result<()> {
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
        Command::List => print!("{}", manager.list()?),
        Command::Update(args) => {
            let outcome = manager.update(
                &args.id_or_name,
                UpdateOptions {
                    target: args.target,
                    icon: args.icon,
                    name: args.name,
                },
            )?;
            match outcome {
                UpdateOutcome::NoChanges { name } => println!(
                    "{}No changes specified for '{}'. Use --target, --name, or --icon to update.{}",
                    geico::colors::BOLD_YELLOW,
                    name,
                    geico::colors::RESET
                ),
                UpdateOutcome::Updated { entry } => println!(
                    "{}Updated '{}' successfully.{}",
                    geico::colors::BOLD_GREEN,
                    entry.name,
                    geico::colors::RESET
                ),
            }
        }
        Command::Remove(args) => manager.remove(&args.id_or_name)?,
    }
    Ok(())
}
