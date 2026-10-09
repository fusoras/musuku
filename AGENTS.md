# Musu

`musu` (結ぶ *musubu* - to bind/tie/link) is a lightweight Linux CLI tool written in Rust designed to integrate portable applications (AppImages, standalone binaries, extracted archive executables, and shell scripts) into the desktop environment and terminal `$PATH`.

## Core Principles

- **Non-destructive**: Never deletes or modifies the original executable file or AppImage.
- **User-space first (No root required)**: Defaults to user directories (`~/.local/share/applications` and `~/.local/bin`), requiring no `sudo` privileges.
- **Selective sudo escalation**: Only escalates with `sudo` if the user explicitly specifies a destination directory outside the user's home directory.
- **Auditable state**: Keeps track of managed entries in `~/.config/musu/apps.toml` for easy listing, auditing, updating, and clean removal.

## Commands Overview

### `musu add <EXECUTABLE_PATH> [OPTIONS]`
Registers an executable file and integrates it into the system.
- `--name <NAME>`: Display name of the application (defaults to executable filename).
- `--icon <PATH>`: Path to the application icon (copied or linked to `~/.local/share/icons/`).
- `--categories <CATEGORIES>`: Desktop categories (e.g., `Development;Utility;`).
- `--bin-name <NAME>`: Custom command name for the terminal symlink.
- `--no-desktop`: Skip creating a `.desktop` launcher entry.
- `--no-symlink`: Skip creating a terminal `$PATH` symlink.
- `--dest-bin <PATH>`: Custom directory for the binary symlink (prompts for `sudo` if outside `$HOME`).
- `--overwrite` / `--force` / `-f`: Replace an existing application with the same name/ID without the interactive menu.

### `musu list`
Lists all managed applications with health checks:
- Displays ID, application name, target binary path, desktop file status, and symlink status.
- Highlights broken links or missing original binaries in color.

### `musu update <ID_OR_NAME> [OPTIONS]`
Updates the metadata, target binary path, or icon of a registered application without recreating it.
- `--target <PATH>`: Update the path to the executable.
- `--icon <PATH>`: Update the application icon.
- `--name <NAME>`: Update the display name.

### `musu remove [ID_OR_NAME...] [OPTIONS]`
Safely removes generated `.desktop` launcher files and terminal symlinks for one or more applications, or all applications with `--all`.
- Supports multiple IDs or names in a single invocation (`musu remove app1 app2 ...`).
- `--all`: Unregisters all applications from `apps.toml` (requires confirmation unless `-y` is passed).
- **Guaranteed safety**: The original AppImage or binary file is **never** deleted unless `--delete-target` is explicitly requested.
- Cleans up the corresponding entries from `apps.toml`.

## Code Navigation & Architecture

- **CodeGraph**: CodeGraph is initialized in this repository. Agents must always use `codegraph` (`codegraph_explore` or CLI) whenever searching for functions, definitions, symbols, or analyzing connections between files.
