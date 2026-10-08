# `config.toml` — musu settings

`musu` reads the optional file `~/.config/musu/config.toml` (or the
equivalent `$XDG_CONFIG_HOME/musu/config.toml`). When missing, all safe
built-in defaults apply. `musu watch <dir>` persists watched directories
into this file, while other commands strictly read it. The file is loaded
for lifecycle operations and for `list`/`show`; malformed files are reported
as errors.

## Display

```toml
[display]
icons = "auto"                 # auto, always, never (alias: images)
list_icon_size = "compact"     # compact, medium, large
show_icon_size = "standard"    # standard, compact
paths = "tilde"                # tilde, absolute
color = "auto"                 # auto, always, never
list_format = "simple"         # simple, table
hide_disabled_symlinks = false # true, false
hide_target_ok = false         # true, false
hide_desktop_ok = false        # true, false
hide_ok_status = false         # true, false (alias: hide_ok)
description = false            # true, false (alias: show_description)
```

| Key | Default | Effect |
| --- | --- | --- |
| `icons` | `"auto"` | Terminal icon previews in compatible terminals (`Kitty`, `Ghostty`, `WezTerm`): `auto` uses terminal detection, `always` forces previews, and `never` disables them. Legacy key `images` is supported as an alias. CLI `--icons` (alias `--image`) forces a preview unless `--no-icons` is present. |
| `list_icon_size` | `"compact"` | Preview footprint for `list`: compact `2x1`, medium `4x2`, or large `8x4` cells. |
| `show_icon_size` | `"standard"` | Preview footprint for `show`: standard `10x5` or compact `6x3`. |
| `paths` | `"tilde"` | Collapses paths below `$HOME` to `~/`; `absolute` keeps full paths. CLI `show --absolute` always wins. |
| `color` | `"auto"` | `auto` honors `NO_COLOR` and `CLICOLOR=0`, `always` emits ANSI styles, and `never` disables them. |
| `list_format` | `"simple"` | Selects the default output of `musu list` (`simple` or `table`). `--table` always selects the aligned table. |
| `hide_disabled_symlinks` | `false` | When `true`, suppresses `symlink=disabled` in simple `list` output. CLI `--hide-disabled-symlinks` takes precedence. |
| `hide_target_ok` | `false` | When `true`, suppresses `target=ok` in simple `list` output when the executable is healthy. Broken or missing targets (`target=missing/broken`) remain visible. CLI `--hide-target-ok` takes precedence. |
| `hide_desktop_ok` | `false` | When `true`, suppresses `desktop=ok` in simple `list` output when the launcher file is healthy. Missing launchers remain visible. CLI `--hide-desktop-ok` takes precedence. |
| `hide_ok_status` | `false` | When `true`, suppresses both `target=ok` and `desktop=ok` when healthy. Accepted alias: `hide_ok`. CLI `--hide-ok` takes precedence. |
| `description` | `false` | When `true`, displays application descriptions in `musu list` output. Accepted aliases: `show_description`, `descriptions`. CLI `--description` forces descriptions; `--no-description` suppresses them. |

The table renderer pads every column to a common width, keeps icon previews
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

## Watch

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

| Key | Default | Effect |
| --- | --- | --- |
| `directories` | `[]` | List of directories monitored for AppImages. Automatically populated when running `musu watch <DIR>`. |
| `icons_dir` | `None` | Optional external directory containing loose icon files matched against AppImage names. |
| `auto_chmod` | `false` | When `true`, automatically grants executable permissions (`+x`) to newly discovered AppImages. |
| `recursive` | `false` | When `true`, scans subdirectories recursively. |
| `clean_on_delete` | `true` | Automatically unregisters and deletes desktop files/symlinks when an AppImage is removed from disk. |
| `auto_sync` | `true` | Triggers transparent live folder synchronization during `musu list`, `musu show`, and `musu run`. |
| `debounce_ms` | `1000` | Debounce interval in milliseconds when running `musu watch --continuous`. |

Unknown keys are ignored. Wrong value types or unsupported enum values produce
a parse error rather than silently changing behavior. The settings file is
independent from `apps.toml`, which stores the managed application registry.


## Sandbox

```toml
[sandbox]
enabled = false
backend = "auto"
```

| Key | Default | Effect |
| --- | --- | --- |
| `enabled` | `false` | When `true`, all newly added or updated applications without an explicit sandbox override will use a sandbox wrapper in their `.desktop` file and symlink. |
| `backend` | `"auto"` | Selects the sandbox backend to use: `"auto"` detects installed ones, `"bwrap"` forces Bubblewrap, `"firejail"` forces Firejail. |
