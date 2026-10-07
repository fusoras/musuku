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
    List(List),
    /// Show detailed information and paths for a registered application.
    #[command(alias = "info")]
    Show(Show),
    /// Update metadata, target executable, icon, or categories of a registered application.
    Update(Update),
    /// Safely remove desktop launcher and symlink (preserves original executable).
    Remove(Remove),
    /// Generate shell completion script (bash, zsh, fish, elvish, powershell).
    Completions(Completions),
    /// Check system environment and desktop integration health.
    Health(Health),
    /// Launch a registered application in the background.
    Run(Run),
    /// Scan and optionally monitor directories for AppImages.
    Watch(Watch),
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
    /// Omit the value to list the standard categories without registering anything.
    #[arg(long, value_delimiter = ',', num_args = 0..)]
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

#[derive(Debug, Args, Default)]
pub struct List {
    /// Display application icon previews in compatible terminals.
    #[arg(short = 'i', long)]
    pub image: bool,
    /// Suppress application icon previews even if supported by the terminal.
    #[arg(long)]
    pub no_image: bool,
    #[arg(short = 't', long)]
    pub table: bool,
    #[arg(long)]
    pub category: Option<String>,
    #[arg(long)]
    pub broken: bool,
}

#[derive(Debug, Args, Default)]
pub struct Health {
    #[arg(long)]
    pub json: bool,
    /// List every checked tool individually (missing ones as dim `[–]`).
    #[arg(short = 'v', long)]
    pub verbose: bool,
}

#[derive(Debug, Args)]
pub struct Run {
    pub id_or_name: String,
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

#[derive(Debug, Args)]
pub struct Watch {
    /// Directories to scan and monitor; defaults to [watch].directories.
    pub directories: Vec<PathBuf>,
    /// Directory containing icons matched by AppImage name.
    #[arg(long)]
    pub icons_dir: Option<PathBuf>,
    /// Scan once and exit (alias: --once).
    #[arg(long, alias = "once")]
    pub scan_only: bool,
    /// Monitor subdirectories recursively.
    #[arg(long)]
    pub recursive: bool,
    /// Show registrations without changing the registry or filesystem.
    #[arg(long)]
    pub dry_run: bool,
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
    /// Omit the value to show the current categories without changing anything.
    #[arg(long, value_delimiter = ',', num_args = 0..)]
    pub categories: Option<Vec<String>>,
    /// Remove the terminal symlink for this application.
    #[arg(long)]
    pub remove_symlink: bool,
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
    /// Print full absolute paths instead of collapsing home directory to ~/.
    #[arg(long, short = 'a', visible_alias = "full-path")]
    pub absolute: bool,
    /// Display application icon preview in the terminal (default in Kitty/Ghostty).
    #[arg(short = 'i', long)]
    pub image: bool,
    /// Suppress terminal image preview even if supported by the terminal.
    #[arg(long)]
    pub no_image: bool,
}

#[derive(Debug, Args)]
pub struct Remove {
    /// ID or name of the registered application to remove.
    pub id_or_name: String,
    /// Also delete the original target executable (AppImage or binary) from disk.
    #[arg(long, visible_alias = "delete-appimage")]
    pub delete_target: bool,
    /// Skip the confirmation prompt when deleting the target executable.
    #[arg(short = 'y', long)]
    pub yes: bool,
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
