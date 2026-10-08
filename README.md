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

### Build from source

Prerequisites:

- A stable Rust toolchain (`rustup` + `cargo`)

```bash
git clone --depth=1 https://github.com/fusoras/geico.git
cd fast-alias
cargo install --path .
```

This installs `geico` into `~/.cargo/bin/`, which is normally already on your `$PATH`.

Verify

```sh
geico --help
```

## Quick start

```sh
# auto-detect icon, name, and register AppImage
geico add ~/NameApp.AppImage

# list all managed applications
geico list --icons

# integrate an AppImage with desktop launcher and custom icon
geico add ~/Downloads/myapp.AppImage --icon ~/Pictures/myapp.png

# update application target executable path
geico update myapp --target ~/Downloads/myapp-v2.AppImage

# set up a live folder (auto-syncs new versions, removals)
geico watch ~/FolderAppimage/
```

```bash
# integrate a binary from tar.xz into terminal PATH only (no desktop file)
geico add /opt/tools/my-cli --no-desktop

# integrate a GUI app without a terminal symlink
geico add /opt/blender/blender --no-symlink --icon /opt/blender/icon.png
```

### Background Watch Service (systemd & SysV)

Run `geico watch` automatically in the background without keeping a terminal open:

```sh
# 1. Install and start the background service (user-space, no root required)
geico watch service install

# 2. Check service status
systemctl --user status geico-watch.service

# SysV alternative:
sudo geico watch service install --sysv
sudo service geico-watch status
```

View live logs:

```sh
journalctl --user -u geico-watch.service -f
```

To stop or remove the service:

```sh
systemctl --user stop geico-watch.service
geico watch service uninstall
```

## Commands

`geico` has nine subcommands. For detailed options, flags, and advanced examples, see the full [Commands Reference](docs/commands.md).

| Command                                                      | Description                                                                        |
| ------------------------------------------------------------ | ---------------------------------------------------------------------------------- |
| [`geico add`](docs/commands.md#1-geico-add)                  | Register an executable and integrate it into desktop and terminal `$PATH`.         |
| [`geico list`](docs/commands.md#2-geico-list)                | List all managed applications with health checks and terminal icon previews.       |
| [`geico show`](docs/commands.md#3-geico-show)                | Inspect full metadata, artifact paths, description, and status for an application. |
| [`geico update`](docs/commands.md#4-geico-update)            | Update target executable path, icon, display name, categories, or description.     |
| [`geico remove`](docs/commands.md#5-geico-remove)            | Safely remove desktop launcher and symlink (preserves original executable).        |
| [`geico watch`](docs/commands.md#6-geico-watch)              | Watch directories for AppImages to auto-register, upgrade, or prune them.          |
| [`geico health`](docs/commands.md#7-geico-health)            | Check environment health, SVG converters, and desktop utilities.                   |
| [`geico run`](docs/commands.md#8-geico-run)                  | Launch a registered application in the background.                                 |
| [`geico completions`](docs/commands.md#10-geico-completions) | Generate or install shell completion scripts for Tab autocompletion.               |

Every subcommand accepts `-h, --help` for instant usage text.

## Documentation

- [`docs/guide.md`](docs/guide.md) — User guide: everyday workflows, desktop integration, and troubleshooting.
- [`docs/commands.md`](docs/commands.md) — Complete command reference, flags, and options cheatsheet.
- [`docs/config.md`](docs/config.md) — Configuration file guide (`~/.config/geico/config.toml`).
- [`docs/watch.md`](docs/watch.md) — Directory watcher and live AppImage sync guide.
