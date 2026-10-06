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
geico list
```

### Output Format

```text
Obsidian (obsidian) target=ok desktop=ok symlink=ok
Blender (blender) target=missing/broken desktop=ok symlink=missing/broken
```

- **`ok` (green)**: File exists and is valid.
- **`missing/broken` (red)**: Target executable was moved or deleted.
- **`disabled` (yellow)**: Artifact was intentionally skipped (via `--no-desktop` or `--no-symlink`).

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

### Examples

```sh
# Full inspection card
geico show obsidian

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

### Examples

```sh
# Point to a newly downloaded AppImage version
geico update obsidian --target ~/Downloads/Obsidian-1.6.0.AppImage

# Rename application
geico update obsidian --name "Obsidian Notes"

# Change categories (supports aliases like dev -> Development)
geico update obsidian --categories Development
```

---

## 5. `geico remove`

Safely removes the `.desktop` launcher, icon, and terminal symlink, and removes the entry from the registry.

```sh
geico remove <ID_OR_NAME>
```

> [!NOTE]
> **Safety Guarantee**: Your original AppImage, binary file, or script is **never deleted**. Only the artifacts created by `geico` are removed.

### Examples

```sh
geico remove obsidian
geico remove "Obsidian Notes"
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
