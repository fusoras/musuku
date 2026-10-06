use anyhow::Result;
use clap::Parser;
use geico::{
    cli::{Cli, Command},
    config, desktop,
    manager::{AddOptions, Manager, UpdateOptions},
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
            manager.update(
                &args.id_or_name,
                UpdateOptions {
                    target: args.target,
                    icon: args.icon,
                    name: args.name,
                },
            )?;
        }
        Command::Remove(args) => manager.remove(&args.id_or_name)?,
    }
    Ok(())
}
