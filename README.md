# musu

Seamless desktop & CLI integration for portable Linux apps

<p align="center">
  <img alt="musu demo" src="https://github.com/user-attachments/assets/48a2578f-cc07-4fe7-a250-f923db963718" width="800" />
</p>

`musu` is a lightweight CLI tool written in Rust that integrates portable
applications — AppImages, standalone binaries (`bin.sh`), and shell scripts — into your desktop environment and your
terminal `$PATH`.

With a single command, `musu` can:

- Create a **desktop launcher** (`.desktop` entry) so the application shows up in
  your application menu.
- Register an **icon** by copying it into `~/.local/share/icons/`.
- Create a **terminal symlink** in `~/.local/bin/` so the command is available in
  every shell.
- Track everything in one **auditable state file**: `~/.config/musu/apps.toml`.

## Core Principles

### User-space first

All defaults live inside your home directory:

| Artifact          | Default location               |
| ----------------- | ------------------------------ |
| Desktop launchers | `~/.local/share/applications/` |
| Icon copies       | `~/.local/share/icons/`        |
| Terminal symlinks | `~/.local/bin/`                |
| State database    | `~/.config/musu/apps.toml`     |

### Auditable TOML state

Every registered application is stored in `~/.config/musu/apps.toml`.
You can open it at any time to see exactly what `musu`
manages, edit it, back it up, or version-control it.

## Installation

### Quick Install

Run the automated installer script:

```sh
curl -fsSL https://raw.githubusercontent.com/uruka1/musu/main/install.sh | bash
```

### Build from source

Prerequisites:

- A stable Rust toolchain (`rustup` + `cargo`)

```bash
git clone --depth=1 https://github.com/uruka1/musu.git
cd musu
cargo install --path .
```

This installs `musu` into `~/.cargo/bin/`, which is normally already on your `$PATH`.

Verify

```sh
musu --help
```

## Quick start

```sh
# auto-detect icon, name, and register AppImage
musu add ~/NameApp.AppImage

# list all managed applications
musu list --icons

# integrate an AppImage with desktop launcher and custom icon
musu add ~/Downloads/myapp.AppImage --icon ~/Pictures/myapp.png

# update application target executable path
musu update myapp --target ~/Downloads/myapp-v2.AppImage

# set up a live folder (auto-syncs new versions, removals)
musu watch ~/FolderAppimage/
```

```bash
# integrate a binary from tar.xz into terminal PATH only (no desktop file)
musu add /opt/tools/my-cli --no-desktop

# integrate a GUI app without a terminal symlink
musu add /opt/blender/blender --no-symlink --icon /opt/blender/icon.png
```

More information: [docs/guide.md](docs/guide.md)

### Background Watch Service (systemd)

`install.sh` automatically creates the user service unit in `~/.config/systemd/user/musu-watch.service`. To run `musu watch` automatically in the background without keeping a terminal open:

```sh
# 1. Enable and start the background service (user-space, no root required)
systemctl --user daemon-reload
systemctl --user enable --now musu-watch.service

# 2. Check service status
systemctl --user status musu-watch.service
```

View live logs:

```sh
journalctl --user -u musu-watch.service -f
```

To stop or disable the service:

```sh
systemctl --user stop musu-watch.service
systemctl --user disable --now musu-watch.service
```

## Commands

`musu` has comprehensive subcommands. For detailed options, flags, and advanced examples, see the full [Commands Reference](docs/commands.md).

| Command                                                    | Description                                                                        |
| ---------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| [`musu add`](docs/commands.md#1-musu-add)                  | Register an executable and integrate it into desktop and terminal `$PATH`.         |
| [`musu list`](docs/commands.md#2-musu-list)                | List all managed applications with health checks and terminal icon previews.       |
| [`musu show`](docs/commands.md#3-musu-show)                | Inspect full metadata, artifact paths, description, and status for an application. |
| [`musu update`](docs/commands.md#4-musu-update)            | Update target executable path, icon, display name, categories, or description.     |
| [`musu remove`](docs/commands.md#5-musu-remove)            | Safely remove desktop launcher and symlink (preserves original executable).        |
| [`musu watch`](docs/commands.md#6-musu-watch)              | Watch directories for AppImages to auto-register, upgrade, or prune them.          |
| [`musu health`](docs/commands.md#7-musu-health)            | Check environment health, SVG converters, and desktop utilities.                   |
| [`musu doctor`](docs/commands.md#8-musu-doctor)            | Diagnose system setup, permissions, directories, and auto-fix issues.              |
| [`musu run`](docs/commands.md#9-musu-run)                  | Launch a registered application in the background.                                 |
| [`musu completions`](docs/commands.md#11-musu-completions) | Generate or install shell completion scripts for Tab autocompletion.               |

Every subcommand accepts `-h, --help` for instant usage text.

## Documentation

- [`docs/guide.md`](docs/guide.md) — User guide: everyday workflows, desktop integration, and troubleshooting.
- [`docs/commands.md`](docs/commands.md) — Complete command reference, flags, and options cheatsheet.
- [`docs/config.md`](docs/config.md) — Configuration file guide (`~/.config/musu/config.toml`).
- [`docs/commands.md#6-musu-watch`](docs/commands.md#6-musu-watch) — Directory watcher and live AppImage sync guide.
