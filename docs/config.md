# `config.toml` — geico settings

`geico` reads the optional file `~/.config/geico/config.toml` (or the
equivalent `$XDG_CONFIG_HOME/geico/config.toml`). It never writes this file.
When it is missing, all defaults below apply. The file is loaded for lifecycle
operations and for `list`/`show`; malformed files are reported as errors.

## Display

```toml
[display]
images = "auto"          # auto, always, never
list_icon_size = "compact" # compact, medium, large
show_icon_size = "standard" # standard, compact
paths = "tilde"           # tilde, absolute
color = "auto"            # auto, always, never
list_format = "simple"    # simple, table
```

| Key | Default | Effect |
| --- | --- | --- |
| `images` | `"auto"` | Kitty/Ghostty previews: `auto` uses terminal detection, `always` forces previews, and `never` disables them. CLI `--image` forces a preview unless `--no-image` is also present. |
| `list_icon_size` | `"compact"` | Preview footprint for `list`: compact `2x1`, medium `4x2`, or large `8x4` cells. |
| `show_icon_size` | `"standard"` | Preview footprint for `show`: standard `10x5` or compact `6x3`. |
| `paths` | `"tilde"` | Collapses paths below `$HOME` to `~/`; `absolute` keeps full paths. CLI `show --absolute` always wins. |
| `color` | `"auto"` | `auto` honors `NO_COLOR` and `CLICOLOR=0`, `always` emits ANSI styles, and `never` disables them. |
| `list_format` | `"simple"` | Selects the default output of `geico list`. `--table` always selects the aligned table. |

The table renderer pads every column to a common width, keeps image previews
in the `ICON` cell, applies the same color policy as the simple view, and uses
dim gray borders for structure.

## Defaults

```toml
[defaults]
categories = ["Utility"]
auto_extract = true
update_desktop_db = true
dest_bin = "~/.local/bin"
symlink_appimages = false
```

| Key | Default | Effect |
| --- | --- | --- |
| `categories` | `["Utility"]` | Fallback desktop category when `add` receives no categories and AppImage metadata has none. CLI `--categories` and extracted metadata take precedence. |
| `auto_extract` | `true` | Extract embedded AppImage `.desktop` metadata and icons during `add`. Set to `false` to skip extraction. |
| `update_desktop_db` | `true` | Silently invokes `update-desktop-database` after add, update, or remove when the utility is installed. |
| `dest_bin` | `"~/.local/bin"` | Default destination for terminal symlinks. `add --dest-bin` takes precedence; `~` expands to the configured home directory. |
| `symlink_appimages` | `false` | Controls PATH symlinks for AppImages only. The default skips them; set `true` to create them. The alias `appimage_symlinks` is also accepted. `--no-symlink` always wins. |

Unknown keys are ignored. Wrong value types or unsupported enum values produce
a parse error rather than silently changing behavior. The settings file is
independent from `apps.toml`, which stores the managed application registry.
