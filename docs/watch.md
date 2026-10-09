# `musu watch`

`musu watch` manages "live" AppImage directories. Rather than requiring a persistent, blocking terminal process, `musu` offers transparent, on-demand synchronization:
- **Clean on delete**: When an AppImage is deleted, its registry entry, `.desktop` launcher, and terminal symlink are automatically removed.
- **Autonomous version upgrades**: When an existing AppImage is replaced by or accompanied by a newer version (e.g. `sonora-v0.38.0-x86_64.AppImage` -> `sonora-v0.42.1-x86_64.AppImage`), `musu` autonomously updates the registration, retargets the `.desktop` launcher and terminal symlink, and re-extracts the application icon without duplicating entries.
- **Automatic detection on `musu list`**: Any configured watch directory is automatically synchronized in milliseconds every time `musu list`, `musu show`, or `musu run` is executed.
- **Fast one-shot sync**: Calling `musu watch [DIRECTORIES...]` synchronizes the folders immediately and exits. It automatically persists new directories to `config.toml` so future commands keep them synchronized.

```sh
# Define and synchronize a directory as a live watch folder
musu watch ~/Applications

# Synchronize current directory
musu watch .

# Preview what would be synchronized without writing changes
musu watch ~/Applications --dry-run

# Remove a directory from the watch list
musu watch --unwatch ~/Applications

# Run persistent foreground watcher loop (optional)
musu watch ~/Applications --continuous
```

Supported options:

| Option | Description |
| --- | --- |
| `DIRECTORIES...` | Directories to synchronize and add to watch list (accepts `.`, relative, or absolute paths). Defaults to `[watch].directories` in `config.toml`. |
| `--unwatch <DIR...>`, `--remove-dir` | Remove specified directories from `[watch].directories` in `config.toml`. |
| `--continuous`, `--foreground` | Keep running a debounced filesystem watcher in foreground. Default is one-shot sync and exit. |
| `--scan-only`, `--once` | Scan once and exit (default behavior). |
| `--icons-dir <DIR>` | Match icons by normalized AppImage name (`png`, `svg`, `webp`, `xpm`, or `ico`). |
| `--recursive` | Include subdirectories. |
| `--dry-run` | Report candidates without registering or updating them. |

## Background Service (systemd & SysV)

Instead of leaving a terminal process running with `--continuous`, you can manage a native background service:

```sh
# Install and start user systemd service (default, no root needed)
musu watch service install

# Check status natively
systemctl --user status musu-watch.service
# or via musu:
musu watch service status

# Follow logs in real time
journalctl --user -u musu-watch.service -f

# Uninstall cleanly
musu watch service uninstall
```

For SysVinit environments, pass `--sysv`:

```sh
sudo musu watch service install --sysv
sudo service musu-watch status
```

Configuration in `~/.config/musu/config.toml`:

```toml
[watch]
directories = ["~/Applications"]
icons_dir = "~/Applications/Icons"
auto_chmod = false
recursive = false
clean_on_delete = true
auto_sync = true
debounce_ms = 1000
```
