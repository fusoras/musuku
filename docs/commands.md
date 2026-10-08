# Geico Commands Reference

Complete command-line reference, options, flags, and advanced examples for `geico`.

```text
Usage: geico <COMMAND>

Commands:
  add          Register an executable and integrate it into desktop and PATH
  list         List all managed applications with health checks
  show         Show detailed information and paths for a registered application
  update       Update metadata, target executable, icon, categories, or description
  remove       Safely remove desktop launcher and symlink (preserves original executable)
  watch        Watch directories for AppImages to auto-register, update, or prune
  health       Check system environment, SVG converters, and desktop integration health
  run          Launch a registered application in the background
  completions  Generate or install shell completion scripts
```

Every command supports `-h, --help` for instant usage text.

---

## 1. `geico add`

Registers an executable file and integrates it into the desktop environment and terminal `$PATH`.

```sh
geico add [OPTIONS] <EXECUTABLE>
```

### Options

| Option | Description | Default |
| --- | --- | --- |
| `<EXECUTABLE>` | Path to the AppImage, binary, or script (must exist, be a regular file, and be executable). | *required* |
| `--name <NAME>` | Display name shown in desktop application menus. | Derived from executable filename |
| `--icon <PATH>` | Path to the icon image (copied to `~/.local/share/icons/`). | Extracted from AppImage or None |
| `--categories <LIST>` | Desktop categories separated by commas (e.g. `Development,Utility`). Use without a value to list standard categories (read-only query). | Extracted or configured defaults |
| `--description <TEXT>` | Application description or tooltip written to `Comment=` in the `.desktop` file (visible alias: `--comment`). | Extracted from AppImage or None |
| `--bin-name <NAME>` | Custom command name for the terminal symlink. | Application ID (slug) |
| `--no-desktop` | Skip creating the `.desktop` launcher entry. | `false` |
| `--no-symlink` | Skip creating the terminal `$PATH` symlink. | `false` |
| `--dest-bin <PATH>` | Custom directory for the binary symlink (prompts for `sudo` if outside `$HOME`). | `~/.local/bin` |

### Fallback Name & ID Behavior

- If `--name` is omitted, the display name is derived from the executable filename, stripping the `.AppImage` extension case-insensitively (e.g. `Firefox.AppImage` → `Firefox`, `/opt/blender-4.2/blender` → `blender`).
- The **ID** is always the slugified display name: ASCII lowercase, with consecutive non-alphanumeric characters replaced by hyphens (e.g. `My Cool App!` → `my-cool-app`).
- The ID determines the `.desktop` filename (`my-cool-app.desktop`), the default symlink name, and the lookup key for commands like `update`, `show`, `run`, and `remove`.

### AppImage Extraction & Symlink Defaults

- For Type 2 AppImages, when `--name`, `--icon`, `--categories`, or `--description` are omitted, `geico` inspects the embedded `.desktop` file and icon via `--appimage-extract`.
- By default, AppImages do **not** receive terminal symlinks to prevent polluting terminal `$PATH` with graphical apps, while standalone binaries and scripts always receive them.
- To enable terminal symlinks for AppImages by default, configure `~/.config/geico/config.toml`:

```toml
[defaults]
symlink_appimages = true
```

The alias `appimage_symlinks = true` is also accepted. Explicit `--no-symlink` always overrides configuration.

### Examples

```sh
# Auto-detect icon, name, description, and register AppImage
geico add ~/Downloads/Obsidian.AppImage

# Full integration with explicit metadata
geico add ~/Downloads/myapp.AppImage \
  --name "My App" \
  --icon ~/Pictures/myapp.png \
  --categories AudioVideo,Utility \
  --description "Lightweight audio recorder"

# Standalone CLI binary: symlink only, no desktop entry
geico add /opt/tools/my-cli --no-desktop --bin-name mycli

# Standalone GUI app: desktop launcher only, no terminal symlink
geico add /opt/blender/blender --no-symlink --icon /opt/blender/icon.png
```

---

## 2. `geico list`

Lists all managed applications with health checks, format options, and terminal icon previews.

```sh
geico list [OPTIONS]
```

### Options

| Option | Description |
| --- | --- |
| `-i, --icons` | Display inline icon previews in compatible terminals using Kitty graphics protocol (alias: `--image`). |
| `--no-icons` | Suppress icon previews even in compatible terminals (alias: `--no-image`). |
| `-t, --table` | Format output as an aligned table with `ICON`, `NAME`, `ID`, `CATEGORIES`, and `STATUS` columns. |
| `--category <CAT>` | Filter applications by desktop category (case-insensitive substring/prefix match). |
| `--broken` | Show only applications whose target binary or artifacts are missing/broken. |
| `--hide-disabled-symlinks` | Hide symlink status indicator when intentionally disabled (`symlink=disabled`). Visible alias: `--hide-disabled`. |
| `--hide-target-ok` | Hide target status when healthy (`target=ok`). |
| `--hide-desktop-ok` | Hide desktop file status when healthy (`desktop=ok`). |
| `--hide-ok` | Hide both target and desktop statuses when healthy. Visible alias: `--hide-ok-status`. |
| `--description` | Display application descriptions in list output (aliases: `--descriptions`, `--desc`). |
| `--no-description` | Suppress application descriptions even if enabled in `config.toml` (aliases: `--no-descriptions`, `--no-desc`). |

### Status Indicators

- **`ok` (green)**: The path exists on disk and is healthy.
- **`missing/broken` (red)**: The target executable or generated artifact was moved or deleted.
- **`disabled` (yellow)**: The artifact was intentionally skipped (via `--no-desktop` or `--no-symlink`).

### Examples

```sh
# Default simple list
geico list

# List with inline terminal icon previews
geico list --icons

# List showing application descriptions
geico list --description

# Aligned table view with descriptions
geico list --table --description

# Filter only broken or missing applications
geico list --broken

# Hide healthy statuses for a cleaner compact view
geico list --hide-ok --hide-disabled
```

---

## 3. `geico show` (alias: `info`)

Inspects the metadata, artifact paths, description, and health status of a registered application.

```sh
geico show <ID_OR_NAME> [OPTIONS]
```

### Options

| Option | Description |
| --- | --- |
| `<ID_OR_NAME>` | Application ID or display name (case-insensitive). |
| `--bin` | Print only the target executable path. |
| `--desktop` | Print only the `.desktop` launcher path. |
| `--icon` | Print only the copied icon path. |
| `--symlink` | Print only the terminal symlink path. |
| `--json` | Print the complete application entry as formatted JSON. |
| `-a, --absolute` | Print full absolute paths instead of collapsing `$HOME` to `~` (alias: `--full-path`). |
| `-i, --icons` | Display application icon preview in the terminal (alias: `--image`). |
| `--no-icons` | Suppress terminal image preview (alias: `--no-image`). |

### Output Format

```text
My App (my-app)
  Executable:  ~/Downloads/myapp [ok]
  Desktop:     ~/.local/share/applications/my-app.desktop [ok]
  Icon:        ~/.local/share/icons/my-app.png [ok]
  Symlink:     ~/.local/bin/my-app [ok]
  Categories:  AudioVideo, Utility
  Description: Lightweight audio recorder
  Registered:  2026-10-06 14:00:00 UTC
```

### Path Extraction for Scripts

```sh
# Open the generated desktop file directly in your editor
nano $(geico show my-app --desktop)

# Inspect the raw JSON entry
geico show my-app --json | jq .
```

---

## 4. `geico update`

Updates the metadata, target executable path, icon, categories, or description of a registered application without re-registering it.

```sh
geico update <ID_OR_NAME> [OPTIONS]
```

### Options

| Option | Description | Default |
| --- | --- | --- |
| `<ID_OR_NAME>` | ID or display name of the application to update. | *required* |
| `--target <PATH>` | New path to the target executable (validated like `add`). | target unchanged |
| `--icon <PATH>` | New icon file (removes previously copied icon first). | icon unchanged |
| `--name <NAME>` | New display name (re-derives slug ID and updates desktop entry). | name unchanged |
| `--categories <LIST>` | New desktop categories (e.g. `Graphics,3DGraphics`). Use without a value to show current categories (read-only query). | categories unchanged |
| `--description <TEXT>` | New description or tooltip for the application (visible alias: `--comment`). | description unchanged |
| `--remove-symlink` | Delete the terminal symlink and clear `symlink` in registry. | symlink kept |

> [!NOTE]
> Modifying `--target`, `--name`, `--icon`, `--categories`, or `--description` regenerates the `.desktop` file and re-points the terminal symlink automatically when those artifacts exist.
> `--remove-symlink` deletes the symlink file without touching the desktop launcher, icon, or original binary.

### Examples

```sh
# Point application to a newly downloaded AppImage version
geico update my-app --target ~/Downloads/myapp-v2.AppImage

# Rename application
geico update my-app --name "My App Pro"

# Update application description
geico update my-app --description "New updated audio suite"

# Update categories
geico update my-app --categories AudioVideo,Development

# Remove terminal symlink while keeping desktop launcher
geico update my-app --remove-symlink
```

---

## 5. `geico remove`

Safely removes the generated `.desktop` launcher, copied icon, and terminal symlink, and drops the entry from `apps.toml`.

```sh
geico remove [ID_OR_NAME...] [OPTIONS]
```

### Options

| Option | Description | Default |
| --- | --- | --- |
| `[ID_OR_NAME...]` | One or more IDs or names of registered applications to remove. | *required unless `--all`* |
| `--all` | Remove all registered applications from the registry (prompts for confirmation). | `false` |
| `--delete-target` | Also delete the original executable/AppImage from disk (alias: `--delete-appimage`). Prompts for confirmation. | `false` |
| `-y, --yes` | Skip confirmation prompts (required for scripts/non-interactive stdin). | `false` |

> [!NOTE]
> **Safety Guarantee**: Without `--delete-target`, your original AppImage, binary file, or script is **never deleted**. Only the artifacts created by `geico` are removed.
> When `--delete-target` is used, `geico` asks for explicit confirmation before touching the executable on disk.

### Examples

```sh
# Remove single application by ID or name
geico remove my-app

# Remove multiple applications at once
geico remove app-one app-two app-three

# Remove all applications
geico remove --all -y

# Complete uninstall including deleting the original executable from disk
geico remove my-app --delete-target
```

---

## 6. `geico watch`

Scans directories for AppImages and registers new applications. Without `--scan-only`, launches a live debounced filesystem monitor that automatically syncs additions, version upgrades, and removals.

```sh
geico watch [DIRECTORIES...] [OPTIONS]
```

### Options

| Option | Description | Default |
| --- | --- | --- |
| `[DIRECTORIES...]` | Directories to scan and monitor. If omitted, uses `[watch].directories` from config. | None |
| `--scan-only` | Run the scan once and exit without launching the continuous filesystem watcher (alias: `--once`). | `false` |
| `-r, --recursive` | Search and monitor subdirectories recursively. | `false` |
| `--icons-dir <DIR>` | Directory to search for matching icons (matched by stem/normalized application name). | None |
| `--dry-run` | Preview what would be added, updated, or removed without modifying the registry. | `false` |

See [`docs/watch.md`](watch.md) for full configuration, debounce settings, and lifecycle details.

### Examples

```sh
# Watch the current directory
geico watch .

# Watch a specific folder continuously
geico watch ~/Applications/

# Scan once without leaving a background watcher
geico watch ~/Downloads/ --scan-only

# Preview changes with dry-run
geico watch ~/Applications/ --dry-run
```

### Background Service (`geico watch service`)

Manage a persistent background watcher using systemd (user service) or SysVinit:

```sh
geico watch service <SUBCOMMAND> [OPTIONS]
```

| Subcommand | Description |
| --- | --- |
| `install` | Install and start background service (`--now=false` to skip auto-start, `--sysv` for SysVinit). |
| `uninstall` | Stop, disable, and remove the background service. |
| `status` | Show live service status. |
| `start` | Start the background service. |
| `stop` | Stop the background service. |
| `restart` | Restart the background service. |

```sh
# Install and start immediately
geico watch service install

# Check status
systemctl --user status geico-watch.service
# or
geico watch service status
```

---

## 7. `geico health`

Reports available SVG converters, terminal Kitty Graphics Protocol support, desktop utilities, environment paths, and healthy/broken registry entries.

```sh
geico health [OPTIONS]
```

### Options

| Option | Description |
| --- | --- |
| `--json` | Output the complete health report as machine-readable JSON. |
| `-v, --verbose` | List every individual tool in capability groups (missing tools shown as dim `[-]`). |

### SVG Converter Priority

SVG icon previews are rasterized with the first converter available in this priority order:
1. `rsvg-convert` (`librsvg2-bin`) — fastest startup, native GNOME rendering.
2. `magick` (ImageMagick 7) — general-purpose fallback.
3. `convert` (ImageMagick 6) — legacy fallback subject to `policy.xml`.

If none is installed, previews are skipped silently without errors and `.desktop` launchers continue to work normally.

### Examples

```sh
# Capability summary
geico health

# Detailed per-tool inventory
geico health --verbose

# JSON export
geico health --json
```

---

## 8. `geico doctor`

Audits the environment and application registry to detect missing directories, broken symlinks, missing executables, and orphaned desktop files. Optionally repairs these issues automatically.

```sh
geico doctor [OPTIONS]
```

### Options

| Option | Description |
| --- | --- |
| `--fix` | Automatically repair detected issues (prune broken symlinks, clean dead entries). |
| `-y, --yes` | Confirm `--fix` operations without interactive prompts. |

### Examples

```sh
# Run diagnostics
geico doctor

# Auto-repair issues and confirm all prompts
geico doctor --fix --yes
```

---

## 9. `geico run`

Launches a registered application in the background, detached from the current terminal, passing through any additional arguments.

```sh
geico run <ID_OR_NAME> [ARGS...]
```

### Examples

```sh
# Launch application in background
geico run obsidian

# Launch with pass-through flags
geico run obsidian --new-window ~/Notes/
```

---

## 10. Categories Query Mode (`--categories` without a value)

`--categories` accepts no value as a read-only query mode:
- `geico update <app> --categories`: prints the application's current categories followed by the standard FreeDesktop category table.
- `geico add <path> --categories`: prints only the standard FreeDesktop category table.

Values that do not match a standard category (or a known alias) are kept as custom categories, with an advisory `Did you mean:` warning on stderr.

```sh
# Query categories for an existing app
geico update my-app --categories

# View standard categories table
geico add ~/bin/tool --categories
```

---

## 11. `geico completions`

Generates shell completion scripts for Tab autocompletion.

```sh
geico completions [SHELL] [OPTIONS]
```

- Supported shells: `bash`, `zsh`, `fish`, `elvish`, `powershell` (auto-detected if omitted).

### Options

| Option | Description |
| --- | --- |
| `--install` | Automatically install completions into your user configuration. |
| `--stdout` | Print raw completion script to stdout. |

### Examples

```sh
# Install completions automatically for current shell
geico completions --install

# Export completions for Fish shell
geico completions fish --stdout > ~/.config/fish/completions/geico.fish
```
