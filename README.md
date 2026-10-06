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

| Artifact            | Default location                          |
| ------------------- | ----------------------------------------- |
| Desktop launchers   | `~/.local/share/applications/`            |
| Icon copies         | `~/.local/share/icons/`                   |
| Terminal symlinks   | `~/.local/bin/`                           |
| State database      | `~/.config/geico/apps.toml`               |

No `sudo` is needed for typical use.

### Selective sudo escalation

`geico` escalates **only** when you explicitly point the symlink destination
outside your home directory with `--dest-bin`, *and* that directory is not
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
curl -fsSL https://raw.githubusercontent.com/fusoras/geico/main/install.sh | bash
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

```sh
# integrate an AppImage with desktop launcher and custom icon
geico add ~/Downloads/myapp.AppImage --icon ~/Pictures/myapp.png

# integrate a binary from tar.xz into terminal PATH only (no desktop file)
geico add /opt/tools/my-cli --no-desktop

# integrate a GUI app without a terminal symlink
geico add /opt/blender/blender --no-symlink --icon /opt/blender/icon.png

# list all managed applications with health checks
geico list

# update application target executable path
geico update myapp --target ~/Downloads/myapp-v2.AppImage

# safely remove desktop launcher and symlink (original binary is NEVER deleted)
geico remove myapp
```

## Commands

`geico` has four subcommands: [`add`](#geico-add), [`list`](#geico-list),
[`update`](#geico-update) and [`remove`](#geico-remove). Every subcommand accepts
`-h, --help` for its own usage text.

### `geico add <EXECUTABLE> [OPTIONS]`

Registers an executable file and integrates it into the system.

| Option                | Description                                                                    | Default                          |
| --------------------- | ------------------------------------------------------------------------------ | -------------------------------- |
| `<EXECUTABLE>`        | Path to the executable file (AppImage, binary, or script). Must exist, be a regular file, and be executable. | required                         |
| `--name <NAME>`       | Display name of the application.                                               | executable filename (see below)  |
| `--icon <PATH>`       | Icon file; **copied** to `~/.local/share/icons/` (source is not moved).        | no icon                          |
| `--categories <LIST>` | Desktop categories separated by commas (e.g. `Development,Utility`).           | no `Categories=` line            |
| `--bin-name <NAME>`   | Custom command name for the terminal symlink.                                  | the application id (slug)        |
| `--no-desktop`        | Skip creating a `.desktop` launcher entry.                                     | desktop entry is created         |
| `--no-symlink`        | Skip creating a terminal `$PATH` symlink.                                      | symlink is created               |
| `--dest-bin <PATH>`   | Custom directory for the binary symlink; prompts for `sudo` if outside `$HOME`. | `~/.local/bin`                   |

**Fallback name behavior:** if `--name` is not given, the display name is the
final component of the executable path, excluding a trailing `.appimage`
extension case-insensitively — e.g. `/opt/blender-4.2/blender` →
`blender`, `~/Downloads/Firefox.AppImage` → `Firefox`. The **id** is always the
slugified display name: ASCII lowercase, runs of non-alphanumeric
characters collapsed to `-` (e.g. `My Cool App!` → `my-cool-app`). The id is used
for the `.desktop` filename (`my-cool-app.desktop`), as the default symlink name,
and as a lookup key for `update`/`remove`.

By default `add` creates **both** a desktop launcher and a terminal symlink; use
`--no-desktop` or `--no-symlink` to create only one of them.

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

### `geico update <ID_OR_NAME> [OPTIONS]`

Updates the metadata, target binary path, or icon of a registered application
without re-registering it. The application is looked up by id **or** display name,
case-insensitively.

| Option                | Description                                                       | Default              |
| --------------------- | ----------------------------------------------------------------- | -------------------- |
| `<ID_OR_NAME>`        | ID or name of the registered application to update.               | required             |
| `--target <PATH>`     | New path to the executable (validated exactly like `add`).        | target unchanged     |
| `--icon <PATH>`       | New icon file; the previously copied icon is removed first.       | icon unchanged       |
| `--name <NAME>`       | New display name; the id is re-derived (slug) from it.            | name unchanged       |
| `--categories <LIST>` | New desktop categories (e.g. `AudioVideo,Development`).           | categories unchanged |

Changing `--target`, `--name`, `--icon`, or `--categories` regenerates the `.desktop` file and
re-points the terminal symlink automatically when those artifacts exist.
Running `update` without any of these options displays a notice and leaves the
application unchanged.

```sh
# point to a new AppImage release
geico update my-app --target ~/Downloads/myapp-v2.AppImage

# rename an app (also renames its .desktop file and symlink)
geico update "My Tool" --name "My Tool Pro"

# assign categories (with smart alias normalization, e.g. dev -> Development)
geico update my-app --categories Development,Utility
```

### `geico remove <ID_OR_NAME>`

Safely removes the generated `.desktop` launcher, the copied icon, and the
terminal symlink, then drops the entry from `apps.toml`. The lookup is
case-insensitive by id or name.

**The original AppImage or binary file is never deleted.**

```sh
geico remove my-app        # by id
geico remove "My App"      # by display name
```

### `geico categories`

Displays the predefined FreeDesktop application categories with human-readable descriptions and common examples. Useful for discovering the right categories to use with `--categories`.

```sh
geico categories
```

### `geico completions <SHELL>`

Generates a shell completion script for tab autocompletion (`bash`, `zsh`, `fish`, `elvish`, `powershell`).

```sh
# Example for Bash
geico completions bash > ~/.local/share/bash-completion/completions/geico

# Example for Fish
geico completions fish > ~/.config/fish/completions/geico.fish

## Documentation

- [`docs/architecture.md`](docs/architecture.md) — detailed design: FreeDesktop/XDG
  integration, directory layout, sudo policy, and the `apps.toml` schema.
- [`docs/cli-reference.md`](docs/cli-reference.md) — per-command reference with
  examples and troubleshooting.

## Development

```sh
cargo test    # unit + integration tests (isolated in temp dirs)
cargo clippy  # lints (clippy::all = warn, unsafe_code = forbid)
```

The project uses Rust edition 2024 and forbids `unsafe` code entirely.
