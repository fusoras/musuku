# geico

Generate Icons & desktop integration

`geico` is a lightweight Linux CLI tool written in Rust that integrates portable
applications — AppImages, standalone binaries (for example those extracted from
`tar.xz` archives), and shell scripts — into your desktop environment and your
terminal `$PATH`.

With a single command, `geico` can:

- Create a **desktop launcher** (`.desktop` entry) so the application shows up in
  your application menu.
- Register an **icon** by copying it into `~/.local/share/icons/`.
- Create a **terminal symlink** in `~/.local/bin/` so the command is available in
  every shell.
- Track everything in one **auditable state file**: `~/.config/geico/apps.toml`.

## Core Principles

### User-space first (no root required)

All defaults live inside your home directory:

| Artifact          | Default location               |
| ----------------- | ------------------------------ |
| Desktop launchers | `~/.local/share/applications/` |
| Icon copies       | `~/.local/share/icons/`        |
| Terminal symlinks | `~/.local/bin/`                |
| State database    | `~/.config/geico/apps.toml`    |

No `sudo` is needed for typical use.

### Selective sudo escalation

`geico` escalates **only** when you explicitly point the symlink destination
outside your home directory with `--dest-bin`, _and_ that directory is not
writable by your user. In that case it runs `sudo ln -sf …` (or `sudo rm -f …`
during removal). Inside `$HOME`, `geico` never invokes `sudo` — a permission
problem there is reported as an error instead.

### Auditable TOML state

Every registered application is stored in `~/.config/geico/apps.toml`.
You can open it at any time to see exactly what `geico`
manages, edit it, back it up, or version-control it.

## Installation

### Quick Install (Recommended)

Run the automated installer script:

```sh
curl -fsSL https://raw.githubusercontent.com/uruka1/geico/main/install.sh | bash
```

Or from a cloned repository:

```sh
./install.sh
```

The script installs `geico` into `~/.local/bin/geico`, automatically wires shell completions, and prints Bun-style configuration guidance if manual `$PATH` setup is needed.

### Build from source

Prerequisites:

- A stable Rust toolchain (`rustup` + `cargo`)

```sh
cargo build --release
```

The binary is produced at `target/release/geico` (stripped and LTO-optimized for small size).

### Install with Cargo

```sh
cargo install --path .
```

This installs `geico` into `~/.cargo/bin/`, which is normally already on your `$PATH`.

### Verify

```sh
geico --help
```

## Quick start

### APPIMAGE

```sh
geico add ~/NameApp.AppImage

# list all managed applications with health checks
geico list

# integrate an AppImage with desktop launcher and custom icon
geico add ~/Downloads/myapp.AppImage --icon ~/Pictures/myapp.png

# update application target executable path
geico update myapp --target ~/Downloads/myapp-v2.AppImage

# safely remove desktop launcher and symlink (original binary is NEVER deleted)
geico remove myapp
```

### Binary

```bash
# integrate a binary from tar.xz into terminal PATH only (no desktop file)
geico add /opt/tools/my-cli --no-desktop

# integrate a GUI app without a terminal symlink
geico add /opt/blender/blender --no-symlink --icon /opt/blender/icon.png
```

## Commands

`geico` has nine subcommands: [`add`](#geico-add), [`list`](#geico-list),
[`show`](#geico-show), [`update`](#geico-update), [`remove`](#geico-remove),
[`completions`](#geico-completions), [`health`](#geico-health), [`run`](#geico-run),
and [`watch`](#geico-watch).
Every subcommand accepts
`-h, --help` for its own usage text.

### `geico watch [DIRECTORIES...] [OPTIONS]`

Scans directories for AppImages and registers new applications (e.g. `geico watch .`
or omitted to use `[watch].directories` from config). Without
`--scan-only`, the initial scan is followed by a debounced filesystem monitor.
Use `--recursive` for subdirectories, `--icons-dir <DIR>` to pair icons by
normalized filename, and `--dry-run` to preview registrations. `--once` is an
alias for `--scan-only`. See [`docs/watch.md`](docs/watch.md) for the complete
configuration and lifecycle details.

### `geico add <EXECUTABLE> [OPTIONS]`

Registers an executable file and integrates it into the system.

| Option                | Description                                                                                                                                       | Default                         |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------- |
| `<EXECUTABLE>`        | Path to the executable file (AppImage, binary, or script). Must exist, be a regular file, and be executable.                                      | required                        |
| `--name <NAME>`       | Display name of the application.                                                                                                                  | executable filename (see below) |
| `--icon <PATH>`       | Icon file; **copied** to `~/.local/share/icons/` (source is not moved).                                                                           | no icon                         |
| `--categories <LIST>` | Desktop categories separated by commas (e.g. `Development,Utility`). Omit the value to list the standard categories without registering anything. | no `Categories=` line           |
| `--bin-name <NAME>`   | Custom command name for the terminal symlink.                                                                                                     | the application id (slug)       |
| `--no-desktop`        | Skip creating a `.desktop` launcher entry.                                                                                                        | desktop entry is created        |
| `--no-symlink`        | Skip creating a terminal `$PATH` symlink.                                                                                                         | symlink is created              |
| `--dest-bin <PATH>`   | Custom directory for the binary symlink; prompts for `sudo` if outside `$HOME`.                                                                   | `~/.local/bin`                  |

**Fallback name behavior:** if `--name` is not given, the display name is the
final component of the executable path, excluding a trailing `.appimage`
extension case-insensitively — e.g. `/opt/blender-4.2/blender` →
`blender`, `~/Downloads/Firefox.AppImage` → `Firefox`. The **id** is always the
slugified display name: ASCII lowercase, runs of non-alphanumeric
characters collapsed to `-` (e.g. `My Cool App!` → `my-cool-app`). The id is used
for the `.desktop` filename (`my-cool-app.desktop`), as the default symlink name,
and as a lookup key for `update`/`remove`.

By default `add` creates a desktop launcher and creates terminal symlinks for
regular binaries/scripts. AppImages do not receive a terminal symlink unless
enabled in configuration; use
`--no-desktop` or `--no-symlink` to create only one of them.

For Type 2 AppImages, omitted `--name`, `--icon`, and `--categories` values are
inferred from the embedded desktop entry and icon using `--appimage-extract`.

AppImage symlink behavior can be configured in
`~/.config/geico/config.toml`:

```toml
[defaults]
symlink_appimages = false
```

The equivalent key `appimage_symlinks` is also accepted. This setting affects
only AppImages; regular binaries and scripts continue to receive symlinks.
`--no-symlink` always disables the symlink regardless of configuration.

```sh
# GUI app: desktop launcher + icon + terminal symlink
geico add ~/Downloads/myapp.AppImage --name "My App" --icon ~/Pictures/myapp.png

# CLI tool extracted from a tar.xz: PATH only
geico add ~/tools/bin/jq --no-desktop --bin-name jq
```

### `geico list`

Lists all managed applications with health checks. Each line shows the display
name, id, and the status of the target binary, the `.desktop` file, and the
symlink:

- `ok` (green) — the path exists.
- `missing/broken` (red) — the recorded path no longer exists (e.g. the original
  binary was moved or deleted).
- `disabled` (yellow) — this artifact was intentionally not created
  (`--no-desktop` / `--no-symlink`).

```text
My App (my-app) target=ok desktop=ok symlink=ok
jq (jq) target=ok desktop=disabled symlink=ok
```

Use `geico list --table`, `--category <CAT>`, or `--broken` for aligned output
and focused health views.

### `geico health [--json] [--verbose]`

Reports available SVG converters, terminal Kitty Graphics Protocol support,
desktop utilities, environment paths, and healthy/broken registry entries.
By default each tool group reports its _capability_ — a single `[ok]` line
listing the available tools, or a `[warning]` only when the whole group is
missing. Add `--verbose` (`-v`) for the per-tool inventory, where a missing
tool is shown as a dim `[–]` instead of a warning.

SVG previews use the first available converter in priority order —
`rsvg-convert` (purpose-built, fastest, GNOME-compatible) → `magick` →
`convert` (ImageMagick, slower and subject to its `policy.xml`). With none
installed, previews are skipped silently and launchers keep working.

### `geico run <ID_OR_NAME> [ARGS...]`

Launches a registered application in the background, passing through any
additional arguments.

### `geico show <ID_OR_NAME> [OPTIONS]`

Shows detailed metadata, artifact paths, and health status for an application.
The `info` alias is also accepted. Without a selector flag, it prints a
formatted card; `--json` prints the complete registry entry as JSON.

| Option      | Description                                                    |
| ----------- | -------------------------------------------------------------- |
| `--bin`     | Print only the target executable path.                         |
| `--desktop` | Print only the desktop file path, or an error if disabled.     |
| `--icon`    | Print only the icon path, or an error if disabled.             |
| `--symlink` | Print only the terminal symlink path, or an error if disabled. |
| `--json`    | Print the complete application entry as JSON.                  |

```text
My App (my-app)
  Executable:  /home/you/Downloads/myapp [ok]
  Desktop:     /home/you/.local/share/applications/my-app.desktop [ok]
  Icon:        disabled
  Symlink:     /home/you/.local/bin/my-app [ok]
  Categories:  none
  Registered:  2026-10-06 14:00:00 UTC
```

### `geico update <ID_OR_NAME> [OPTIONS]`

Updates the metadata, target binary path, or icon of a registered application
without re-registering it. The application is looked up by id **or** display name,
case-insensitively.

| Option                | Description                                                                                                                      | Default              |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------------- | -------------------- |
| `<ID_OR_NAME>`        | ID or name of the registered application to update.                                                                              | required             |
| `--target <PATH>`     | New path to the executable (validated exactly like `add`).                                                                       | target unchanged     |
| `--icon <PATH>`       | New icon file; the previously copied icon is removed first.                                                                      | icon unchanged       |
| `--name <NAME>`       | New display name; the id is re-derived (slug) from it.                                                                           | name unchanged       |
| `--categories <LIST>` | New desktop categories (e.g. `AudioVideo,Development`). Omit the value to show the current categories without changing anything. | categories unchanged |
| `--remove-symlink`    | Delete the terminal symlink and clear `symlink` in `apps.toml`.                                                                  | symlink kept         |

Changing `--target`, `--name`, `--icon`, or `--categories` regenerates the `.desktop` file and
re-points the terminal symlink automatically when those artifacts exist.
Running `update` without any of these options displays a notice and leaves the
application unchanged.

`--remove-symlink` deletes the symlink from `~/.local/bin` and records the change in
`apps.toml` while keeping the `.desktop` launcher, icon, and original executable intact.
When combined with `--target`/`--name`, the symlink is **not** regenerated. If the
application has no symlink and nothing else changed, `NoChanges` is reported.

```sh
# point to a new AppImage release
geico update my-app --target ~/Downloads/myapp-v2.AppImage

# rename an app (also renames its .desktop file and symlink)
geico update "My Tool" --name "My Tool Pro"

# assign categories (with smart alias normalization, e.g. dev -> Development)
geico update my-app --categories Development,Utility

# drop the terminal symlink, keep the desktop launcher
geico update my-app --remove-symlink
```

### `geico remove [ID_OR_NAME...] [OPTIONS]`

Safely removes the generated `.desktop` launcher, the copied icon, and the
terminal symlink, then drops the entry from `apps.toml`. The lookup is
case-insensitive by id or name.

**By default the original AppImage or binary file is never deleted.**

| Option            | Description                                                                                                    | Default                 |
| ----------------- | -------------------------------------------------------------------------------------------------------------- | ----------------------- |
| `<ID_OR_NAME...>` | One or more IDs or names of registered applications to remove.                                                 | required unless `--all` |
| `--all`           | Remove all registered applications.                                                                            | off                     |
| `--delete-target` | Also delete the original executable/AppImage from disk (alias: `--delete-appimage`). Prompts for confirmation. | off                     |
| `-y`, `--yes`     | Skip the confirmation prompt (for scripts and automation).                                                     | off                     |

With `--delete-target`, `geico` asks
`Are you sure you want to permanently delete '<path>'? [y/N]: ` before touching the
file; any answer other than `y`/`yes` aborts with `Aborted: executable was not deleted.`
and nothing is removed. When stdin is not an interactive terminal, the command refuses
to guess and exits with a non-zero status unless `--yes` is given.

```sh
geico remove my-app another-app            # remove multiple applications by id
geico remove "My App"                      # by display name
geico remove --all -y                      # remove all registered applications
geico remove my-app --delete-target        # full uninstall, asks first
geico remove my-app --delete-appimage -y   # same, alias, no prompt
```

Without `-y`, `--all` asks for confirmation and refuses to run when stdin is
not interactive. With `--all --delete-target`, the confirmation warns that the
registered target executables will also be permanently deleted.

### Listing categories: `--categories` without a value

`--categories` accepts no value as a read-only query mode. `geico update
<app> --categories` prints the application's current categories followed by
the standard FreeDesktop category table; `geico add <path> --categories`
prints only the table. Neither registers nor modifies anything.

```sh
geico update my-app --categories
geico add ~/tools/bin/jq --categories
```

Values that do not match a standard category (or a known alias) are kept as
custom categories, with an advisory `Did you mean:` warning on stderr:

```text
Unknown category 'juegoz' — kept as custom. Did you mean: Game?
  Run 'geico update <app> --categories' (sin valor) to see all standard categories.
```

### `geico completions <SHELL>`

Generates a shell completion script for tab autocompletion (`bash`, `zsh`, `fish`, `elvish`, `powershell`).

````sh
# Example for Bash
geico completions bash > ~/.local/share/bash-completion/completions/geico

# Example for Fish
geico completions fish > ~/.config/fish/completions/geico.fish

## Documentation

- [`docs/guide.md`](docs/guide.md) — User guide: everyday workflows, desktop integration, and troubleshooting.
- [`docs/cli-reference.md`](docs/cli-reference.md) — Complete command reference and options cheatsheet.

## Development

```sh
cargo test    # unit + integration tests (isolated in temp dirs)
cargo clippy  # lints (clippy::all = warn, unsafe_code = forbid)
````

The project uses Rust edition 2024 and forbids `unsafe` code entirely.
