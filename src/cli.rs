use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "geico",
    about = "Integrate portable applications into the desktop"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Register an executable and integrate it into the desktop and PATH.
    Add(Add),
    /// List all managed applications with health checks.
    List,
    /// Show detailed information and paths for a registered application.
    #[command(alias = "info")]
    Show(Show),
    /// Update metadata, target executable, icon, or categories of a registered application.
    Update(Update),
    /// Safely remove desktop launcher and symlink (preserves original executable).
    Remove(Remove),
    /// List standard FreeDesktop application categories with descriptions and examples.
    Categories,
    /// Generate shell completion script (bash, zsh, fish, elvish, powershell).
    Completions(Completions),
}

#[derive(Debug, Args)]
pub struct Add {
    /// Path to the executable file (AppImage, binary, or script).
    pub executable: PathBuf,
    /// Display name of the application (defaults to executable filename).
    #[arg(long)]
    pub name: Option<String>,
    /// Path to the icon file (copied to ~/.local/share/icons).
    #[arg(long)]
    pub icon: Option<PathBuf>,
    /// Desktop categories separated by commas (e.g. AudioVideo,Development).
    #[arg(long, value_delimiter = ',')]
    pub categories: Option<Vec<String>>,
    /// Custom command name for the terminal symlink.
    #[arg(long)]
    pub bin_name: Option<String>,
    /// Skip creating a .desktop launcher entry.
    #[arg(long)]
    pub no_desktop: bool,
    /// Skip creating a terminal $PATH symlink.
    #[arg(long)]
    pub no_symlink: bool,
    /// Custom destination directory for the symlink (prompts sudo if outside $HOME).
    #[arg(long)]
    pub dest_bin: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct Update {
    /// ID or name of the registered application to update.
    pub id_or_name: String,
    /// New path to the executable file.
    #[arg(long)]
    pub target: Option<PathBuf>,
    /// New path to the icon file.
    #[arg(long)]
    pub icon: Option<PathBuf>,
    /// New display name.
    #[arg(long)]
    pub name: Option<String>,
    /// New desktop categories separated by commas (e.g. AudioVideo,Development).
    #[arg(long, value_delimiter = ',')]
    pub categories: Option<Vec<String>>,
}

#[derive(Debug, Args)]
pub struct Show {
    /// ID or name of the registered application to inspect.
    pub id_or_name: String,
    /// Print only the target executable path.
    #[arg(long)]
    pub bin: bool,
    /// Print only the desktop file path.
    #[arg(long)]
    pub desktop: bool,
    /// Print only the icon path.
    #[arg(long)]
    pub icon: bool,
    /// Print only the terminal symlink path.
    #[arg(long)]
    pub symlink: bool,
    /// Print the application entry as JSON.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct Remove {
    /// ID or name of the registered application to remove.
    pub id_or_name: String,
}

#[derive(Debug, Args)]
pub struct Completions {
    /// Shell to generate or install completions for (defaults to active shell).
    pub shell: Option<clap_complete::Shell>,
    /// Install completion script automatically into user shell directory (default behavior).
    #[arg(long)]
    pub install: bool,
    /// Print raw completion script to stdout instead of installing to disk.
    #[arg(long)]
    pub stdout: bool,
}
