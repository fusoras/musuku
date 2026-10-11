use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "musu",
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
    /// Audit and auto-repair broken symlinks, orphaned desktop files, and sandbox configuration.
    Doctor(Doctor),
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
    /// Replace an existing application with the same name/ID without asking.
    #[arg(short = 'f', long, visible_alias = "force")]
    pub overwrite: bool,
    /// Application description or tooltip (written as Comment in .desktop).
    /// Repeat for per-language comments:
    /// `--description "Default" --description es="En español"`.
    #[arg(
        short = 'd',
        long,
        visible_aliases = ["comment", "desc"],
        value_name = "TEXT | LANG=TEXT"
    )]
    pub description: Vec<String>,
    /// Run the application in an isolated sandbox (bwrap or firejail).
    #[arg(long, visible_alias = "isolate")]
    pub sandbox: bool,
    /// Disable sandboxing explicitly.
    #[arg(long)]
    pub no_sandbox: bool,
    /// Enable or disable network access in the sandbox (overrides global policy).
    #[arg(long, value_name = "true|false", num_args = 1)]
    pub network: Option<bool>,
    /// Mount $HOME writable (true) or read-only (false) in the sandbox (overrides global policy).
    #[arg(long, value_name = "true|false", num_args = 1)]
    pub home_writable: Option<bool>,
    /// Additional path to hide inside the sandbox (repeatable; replaces global hide_paths).
    #[arg(long = "hide-path", value_name = "PATH")]
    pub hide_path: Vec<PathBuf>,
    /// Additional writable bind path inside the sandbox (repeatable).
    #[arg(long = "extra-rw", value_name = "PATH")]
    pub extra_rw: Vec<PathBuf>,
    /// Additional read-only bind path inside the sandbox (repeatable).
    #[arg(long = "extra-ro", value_name = "PATH")]
    pub extra_ro: Vec<PathBuf>,
    /// Save metadata to a portable .musu.toml bundle file alongside the application.
    #[arg(long)]
    pub portable: bool,
}

#[derive(Debug, Args, Default)]
pub struct List {
    /// Sort order: name (a-z, default), name-desc, id, id-desc, created, category.
    #[arg(long, value_enum, default_value = "name")]
    pub sort: crate::manager::SortField,
    /// Display application icon previews in compatible terminals.
    #[arg(long = "icons")]
    pub icons: bool,
    /// Suppress application icon previews even if supported by the terminal.
    #[arg(long = "no-icons")]
    pub no_icons: bool,
    #[arg(short = 't', long)]
    pub table: bool,
    #[arg(long)]
    pub category: Option<String>,
    #[arg(long)]
    pub broken: bool,
    /// Hide symlinks when they are disabled.
    #[arg(long, visible_alias = "hide-disabled")]
    pub hide_disabled_symlinks: bool,
    /// Hide target status when it is ok.
    #[arg(long)]
    pub hide_target_ok: bool,
    /// Hide desktop file status when it is ok.
    #[arg(long)]
    pub hide_desktop_ok: bool,
    /// Hide target and desktop statuses when they are ok.
    #[arg(long, visible_alias = "hide-ok-status")]
    pub hide_ok: bool,
    /// Display application descriptions in list output.
    #[arg(long, visible_alias = "descriptions", visible_alias = "desc")]
    pub description: bool,
    /// Suppress application descriptions even if enabled in configuration.
    #[arg(long, visible_alias = "no-descriptions", visible_alias = "no-desc")]
    pub no_description: bool,
    /// Print only application IDs, one per line (useful for scripting).
    #[arg(long)]
    pub ids: bool,
    /// Output the list of applications as a structured JSON array.
    #[arg(long)]
    pub json: bool,
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
#[command(subcommand_precedence_over_arg(true))]
pub struct Watch {
    /// Directories to scan and monitor; defaults to [watch].directories. Pass '.' for current directory.
    pub directories: Vec<PathBuf>,
    /// Directory containing icons matched by AppImage name.
    #[arg(long)]
    pub icons_dir: Option<PathBuf>,
    /// Keep running filesystem watcher in foreground (alias: --foreground).
    #[arg(long, visible_alias = "foreground")]
    pub continuous: bool,
    /// Scan once and exit (alias: --once).
    #[arg(long, alias = "once")]
    pub scan_only: bool,
    /// Monitor subdirectories recursively.
    #[arg(long)]
    pub recursive: bool,
    /// Show registrations without changing the registry or filesystem.
    #[arg(long)]
    pub dry_run: bool,
    /// Remove one or more directories from the watch list in config.toml
    #[arg(long, value_name = "DIR", num_args = 1.., visible_alias = "remove-dir")]
    pub unwatch: Option<Vec<PathBuf>>,
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
    /// New application description or tooltip.
    /// Repeat for per-language comments:
    /// `--description "Default" --description es="En español"`.
    #[arg(
        short = 'd',
        long,
        visible_aliases = ["comment", "desc"],
        value_name = "TEXT | LANG=TEXT"
    )]
    pub description: Vec<String>,
    /// Run the application in an isolated sandbox (bwrap or firejail).
    #[arg(long, visible_alias = "isolate")]
    pub sandbox: bool,
    /// Disable sandboxing explicitly.
    #[arg(long)]
    pub no_sandbox: bool,
    /// Enable or disable network access in the sandbox (overrides global policy).
    #[arg(long, value_name = "true|false", num_args = 1)]
    pub network: Option<bool>,
    /// Mount $HOME writable (true) or read-only (false) in the sandbox (overrides global policy).
    #[arg(long, value_name = "true|false", num_args = 1)]
    pub home_writable: Option<bool>,
    /// Additional path to hide inside the sandbox (repeatable; replaces global hide_paths).
    #[arg(long = "hide-path", value_name = "PATH")]
    pub hide_path: Vec<PathBuf>,
    /// Additional writable bind path inside the sandbox (repeatable).
    #[arg(long = "extra-rw", value_name = "PATH")]
    pub extra_rw: Vec<PathBuf>,
    /// Additional read-only bind path inside the sandbox (repeatable).
    #[arg(long = "extra-ro", value_name = "PATH")]
    pub extra_ro: Vec<PathBuf>,
    /// Save metadata to a portable .musu.toml bundle file alongside the application.
    #[arg(long)]
    pub portable: bool,
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
    #[arg(long = "icons")]
    pub icons: bool,
    /// Suppress terminal image preview even if supported by the terminal.
    #[arg(long = "no-icons")]
    pub no_icons: bool,
}

#[derive(Debug, Args)]
pub struct Remove {
    /// IDs or names of the registered applications to remove.
    #[arg(required_unless_present = "all")]
    pub id_or_names: Vec<String>,
    /// Remove all registered applications from the system.
    #[arg(long, conflicts_with = "id_or_names")]
    pub all: bool,
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

#[derive(Debug, Args)]
pub struct Doctor {
    /// Automatically repair detected issues (clean up dead entries, prune broken symlinks, regenerate outdated sandbox launchers).
    #[arg(long)]
    pub fix: bool,
    /// Confirm without interactive prompt.
    #[arg(short = 'y', long = "yes")]
    pub yes: bool,
}
