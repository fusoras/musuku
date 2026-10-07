# Geico CLI Reference

Command-line reference and options cheatsheet for `geico`.

```text
Usage: geico <COMMAND>

Commands:
  add          Register an executable and integrate it into the desktop and PATH
  list         List all managed applications with health checks
  show         Show detailed information and paths for a registered application
  update       Update metadata, target executable, icon, or categories
  remove       Safely remove desktop launcher and symlink (preserves original executable)
  categories   List standard FreeDesktop application categories
  completions  Generate or install shell completion scripts
  health       Check system environment and desktop integration health
  run          Launch a registered application in the background
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
| `<EXECUTABLE>` | Path to the AppImage, binary, or script (must exist and be executable). | *required* |
| `--name <NAME>` | Display name shown in application menus. | Filename without `.AppImage` |
| `--icon <PATH>` | Path to the icon image (copied to `~/.local/share/icons/`). | None |
| `--categories <LIST>` | Desktop categories separated by commas (e.g. `Development,Utility`). | None |
| `--bin-name <NAME>` | Custom command name for the terminal symlink. | Application ID |
| `--no-desktop` | Skip creating the `.desktop` menu launcher. | `false` |
| `--no-symlink` | Skip creating the terminal `$PATH` symlink. | `false` |
| `--dest-bin <PATH>` | Custom directory for the binary symlink. | `~/.local/bin` |

When `--icon`, `--categories`, or `--name` are omitted for a Type 2 AppImage,
`geico add` attempts to read the embedded desktop entry and icon via
`--appimage-extract`.

To enable terminal symlinks for AppImages while retaining the default
non-symlink behavior, create `~/.config/geico/config.toml`:

```toml
[defaults]
symlink_appimages = false
```

`appimage_symlinks = false` is accepted as an alias. Missing configuration
defaults to AppImage symlinks disabled; set `symlink_appimages = true` to
enable them, and `--no-symlink` overrides the setting for every executable type.

### Examples

```sh
# AppImage with full integration
geico add ~/Downloads/Obsidian.AppImage \
  --name "Obsidian" \
  --icon ~/Pictures/obsidian.png \
  --categories Office,Utility

# CLI tool without desktop launcher
geico add /opt/tools/my-cli --no-desktop

# GUI app with custom command name
geico add ~/Apps/Blender/blender --bin-name blender-4
```

---

## 2. `geico list`

Lists all managed applications with color-coded health checks.

```sh
geico list [OPTIONS]
```

### Options

| Option | Description |
| --- | --- |
| `-i, --image` | Display application icon previews in compatible terminals (Kitty graphics protocol). |
| `--no-image` | Suppress application icon previews even in compatible terminals. |
| `-t, --table` | Display aligned `ICON`, `NAME`, `ID`, `CATEGORIES`, and `STATUS` columns. |
| `--category <CAT>` | Show only applications containing the category (case-insensitive). |
| `--broken` | Show only applications whose target executable is missing. |

### Output Format

```text
Obsidian (obsidian) target=ok desktop=ok symlink=ok
Blender (blender) target=missing/broken desktop=ok symlink=missing/broken
```

- **`ok` (green)**: File exists and is valid.
- **`missing/broken` (red)**: Target executable was moved or deleted.
- **`disabled` (yellow)**: Artifact was intentionally skipped (via `--no-desktop` or `--no-symlink`).

## `geico health`

Checks SVG converters, Kitty Graphics Protocol terminal support, desktop
utilities, environment paths, and the application registry. Each tool group
is reported by capability: available tools are listed on an `[ok]` line, and
`[warning]` appears only when a whole group is missing. Use `--verbose`
(`-v`) for the per-tool inventory, where missing tools are shown as `[–]`.

```sh
geico health
geico health --verbose
geico health --json
```

## `geico run <ID_OR_NAME> [ARGS...]`

Launches a registered executable with optional arguments, detached from the
current terminal.

```sh
geico run obsidian --new-window
```

---

## 3. `geico show` (alias: `info`)

Inspects the paths, file statuses, and metadata of a registered application.

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
| `--json` | Print the entire application record as formatted JSON. |
| `-a, --absolute` | Print full absolute paths instead of collapsing home directory to `~` (alias: `--full-path`). |
| `-i, --image` | Display application icon preview in the terminal (Kitty graphics protocol). |
| `--no-image` | Suppress the terminal image preview even if supported by the terminal. |

### Examples

```sh
# Full inspection card (collapses home directory to ~/ by default)
geico show obsidian

# Show full absolute paths with username instead of ~/
geico show obsidian --absolute

# Force or toggle image preview
geico show obsidian -i

# Text card only, even in terminals with image preview support
geico show obsidian --no-image

# Use paths in terminal commands or scripts
nano $(geico show obsidian --desktop)

# View raw JSON data
geico show obsidian --json
```

---

## 4. `geico update`

Updates the executable path, icon, display name, or categories of an existing application without re-creating it from scratch.

```sh
geico update <ID_OR_NAME> [OPTIONS]
```

### Options

| Option | Description |
| --- | --- |
| `<ID_OR_NAME>` | Application ID or display name (case-insensitive). |
| `--target <PATH>` | Update path to the executable (e.g. after updating an AppImage). |
| `--icon <PATH>` | Replace the application icon with a new image. |
| `--name <NAME>` | Update the display name (also syncs `.desktop` and default symlink). |
| `--categories <LIST>` | Update desktop categories (e.g. `Graphics,3DGraphics`). |
| `--remove-symlink` | Delete the terminal symlink and clear it from `apps.toml`. |

> [!NOTE]
> `--remove-symlink` removes only the `$PATH` symlink: the `.desktop` launcher, icon, and
> original executable stay untouched. Combined with `--target`/`--name`, the symlink is
> **not** recreated. If the application has no symlink and nothing else changed, `geico`
> reports "No changes".

### Examples

```sh
# Point to a newly downloaded AppImage version
geico update obsidian --target ~/Downloads/Obsidian-1.6.0.AppImage

# Rename application
geico update obsidian --name "Obsidian Notes"

# Change categories (supports aliases like dev -> Development)
geico update obsidian --categories Development

# Remove the terminal symlink but keep the desktop launcher
geico update obsidian --remove-symlink
```

---

## 5. `geico remove`

Safely removes the `.desktop` launcher, icon, and terminal symlink, and removes the entry from the registry.

```sh
geico remove <ID_OR_NAME> [--delete-target|--delete-appimage] [-y|--yes]
```

### Options

| Option | Description |
| --- | --- |
| `<ID_OR_NAME>` | Application ID or display name (case-insensitive). |
| `--delete-target` | Also delete the original executable/AppImage from disk. Visible alias: `--delete-appimage`. |
| `-y`, `--yes` | Skip the confirmation prompt (required when stdin is not interactive). |

> [!NOTE]
> **Safety Guarantee**: Without `--delete-target`, your original AppImage, binary file, or
> script is **never deleted**. Only the artifacts created by `geico` are removed.

> [!IMPORTANT]
> With `--delete-target`, `geico` prompts
> `Are you sure you want to permanently delete '<path>'? [y/N]: `. Any answer other than
> `y`/`yes` prints `Aborted: executable was not deleted.` and removes nothing. When stdin
> is not a terminal (pipes, CI), the command exits with a non-zero status unless `--yes`
> (`-y`) is supplied.

### Examples

```sh
geico remove obsidian
geico remove "Obsidian Notes"

# Complete uninstall including the AppImage (asks for confirmation)
geico remove obsidian --delete-target

# Same, using the alias and skipping the prompt in scripts
geico remove obsidian --delete-appimage --yes
```

---

## 6. `geico categories`

Displays the standard FreeDesktop application categories with descriptions and examples to help you choose the right category tags.

```sh
geico categories
```

---

## 7. `geico completions`

Generates shell completion scripts for Tab autocompletion.

```sh
geico completions [SHELL] [OPTIONS]
```

- Supported shells: `bash`, `zsh`, `fish`, `elvish`, `powershell`.
- Auto-detected if omitted.

### Options

| Option | Description |
| --- | --- |
| `--install` | Automatically install completions into your user configuration. |
| `--stdout` | Print raw completion script to stdout. |

### Examples

```sh
# Install completions automatically for current shell
geico completions --install

# Output completions for fish
geico completions fish --stdout > ~/.config/fish/completions/geico.fish
```
