# `geico watch`

`geico watch` scans one or more directories for AppImages and registers new
files with the normal `geico add` lifecycle. With no `--scan-only`, it keeps a
debounced filesystem watcher running after the initial scan. AppImages remain
in their original directories; when a registered file is deleted, its desktop
entry, symlink, and registry entry are removed when `clean_on_delete` is
enabled.

```sh
# Scan once
geico watch ~/Applications --scan-only

# Scan and monitor recursively, using a dedicated icon folder
geico watch ~/Applications --recursive --icons-dir ~/Applications/Icons

# Preview what would be registered
geico watch ~/Downloads --scan-only --dry-run
```

Supported options:

| Option | Description |
| --- | --- |
| `DIRECTORIES...` | Directories to scan and monitor. Configuration is used when omitted. |
| `--scan-only`, `--once` | Scan and exit without starting the monitor. |
| `--icons-dir <DIR>` | Match icons by normalized AppImage name (`png`, `svg`, `webp`, `xpm`, or `ico`). |
| `--recursive` | Include subdirectories. |
| `--dry-run` | Report candidates without registering them. |

Defaults can be stored in `~/.config/geico/config.toml`:

```toml
[watch]
directories = ["~/Applications"]
icons_dir = "~/Applications/Icons"
auto_chmod = false
recursive = false
clean_on_delete = true
debounce_ms = 1000
```

`auto_chmod` is disabled by default. When enabled, detected AppImages receive
the executable bit before registration. If no matching external icon is found,
the regular AppImage metadata and `.DirIcon` extraction fallback is used.
