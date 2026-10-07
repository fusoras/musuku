# Plan: config.toml options

**Status:** implemented.
**Date:** 2026-10-07
**Source:** options defined by Antigravity and agreed by the user.

All options below are implemented in `src/config.rs`, `src/manager.rs`,
`src/image.rs`, and `src/colors.rs`, and documented in `docs/config.md`.

---

## Planned `~/.config/geico/config.toml`

### `[display]`

Controls the look and feel of `geico list` / `geico show` and terminal output.

| Key | Type | Values | Default | What it does |
|---|---|---|---|---|
| `images` | string | `"auto"` \| `"always"` \| `"never"` | `"auto"` | Controls Kitty/Ghostty image previews: `auto` = only if the terminal supports it, `always` = force, `never` = disable. |
| `list_icon_size` | string | `"compact"` (2x1) \| `"medium"` (4x2) \| `"large"` (8x4) | `"compact"` | Default icon size in `geico list`. |
| `show_icon_size` | string | `"standard"` (10x5) \| `"compact"` (6x3) | `"standard"` | Default icon size in `geico show`. |
| `paths` | string | `"tilde"` (`~/`) \| `"absolute"` (`/home/user/...`) | `"tilde"` | How file paths are formatted in `show`. |
| `color` | string | `"auto"` \| `"always"` \| `"never"` | `"auto"` | Output color policy (respects `NO_COLOR` when `auto`). |
| `list_format` | string | `"simple"` \| `"table"` | `"simple"` | Default list style: `simple` = current compact view, `table` = aligned table with borders. |

### `[defaults]`

Default behavior of `geico add` / lifecycle operations.

| Key | Type | Values | Default | What it does |
|---|---|---|---|---|
| `symlink_appimages` | bool | `true` \| `false` (alias `appimage_symlinks`) | `false` | When `false` (the default), `geico add` skips the `$PATH` symlink for AppImage targets. Set `true` to enable it. |
| `categories` | array of strings | e.g. `["Utility"]` | `["Utility"]` | Fallback desktop category if none specified during `geico add`. |
| `auto_extract` | bool | `true` \| `false` | `true` | Auto-extract embedded icons and `.desktop` metadata from AppImages. |
| `update_desktop_db` | bool | `true` \| `false` | `true` | Silently refresh the desktop database after add/update/remove. |
| `dest_bin` | string | path (supports `~`) | `"~/.local/bin"` | Default directory for terminal symlinks. |

---

## Full example

```toml
[display]
# Controls Kitty/Ghostty image previews: "auto" (if supported), "always", or "never"
images = "auto"

# Default icon size in 'geico list': "compact" (2x1), "medium" (4x2), or "large" (8x4)
list_icon_size = "compact"

# Default icon size in 'geico show': "standard" (10x5) or "compact" (6x3)
show_icon_size = "standard"

# How file paths are formatted in 'show': "tilde" (~/) or "absolute" (/home/user/...)
paths = "tilde"

# Output color policy: "auto", "always", "never"
color = "auto"

# Default list style: "simple" (current compact view) or "table" (aligned table with borders)
list_format = "simple"

[defaults]
# Fallback desktop category if none specified during 'geico add'
categories = ["Utility"]

# Auto-extract embedded icons and .desktop metadata from AppImages
auto_extract = true

# Silently refresh desktop database after add/update/remove
update_desktop_db = true

# Default directory for terminal symlinks
dest_bin = "~/.local/bin"

# Skip $PATH symlink for AppImage targets (default)
symlink_appimages = false
```

---

## Planned improvement: `list_format = "table"` view

The `table` list style needs a proper visual overhaul before it can be considered
usable as the default alternative to `simple`:

- **Column alignment**: all columns must be aligned and share a consistent width —
  every row must render with the same total width (pad cells to the widest entry per
  column; no ragged rows).
- **Images**: the table must support inline icon/image previews in the icon column
  (same cell footprint rules as the `simple` view; honor `display.images` and
  `display.list_icon_size`).
- **Colors**: cell content keeps the existing color policy (`display.color`, `NO_COLOR`).
- **Borders**: the table borders/box drawing must be **gray** so they read as structure
  and not as content.

---

## Implementation checklist (future work)

- [x] Extend `Settings`/`Defaults` structs in `src/config.rs`; add new `[display]` section struct.
- [x] Wire `display.images` into image preview logic (`src/image.rs`) — CLI flags retain precedence.
- [x] Wire `list_icon_size` / `show_icon_size` into `geico list` / `geico show`.
- [x] Wire `paths` into `show` path formatting.
- [x] Wire `color` into color output helpers (`src/colors.rs`), respecting `NO_COLOR`.
- [x] Wire `list_format` into `geico list`.
- [x] Improve `table` view: aligned columns with uniform row width, inline image support, colors, gray borders.
- [x] Wire `defaults.categories` as fallback in `geico add`.
- [x] Wire `defaults.auto_extract` into AppImage extraction.
- [x] Wire `defaults.update_desktop_db` into desktop DB refresh.
- [x] Wire `defaults.dest_bin` as default symlink target dir (CLI `--dest-bin` keeps precedence).
- [x] Add dedicated configuration/display behavior tests; verify RED against the pre-implementation tree, then GREEN with the implementation, and document all options in `docs/config.md` (English).
