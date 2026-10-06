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
            clap_complete::generate(
                args.shell,
                &mut Cli::command(),
                "geico",
                &mut std::io::stdout(),
            );
        }
    }
    Ok(())
}
