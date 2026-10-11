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
| `icons` | `"auto"` | Terminal icon previews in compatible terminals (`Kitty`, `Ghostty`, `WezTerm`): `auto` uses terminal detection, `always` forces previews, and `never` disables them. Legacy key `images` is supported as an alias. CLI `--icons` forces a preview unless `--no-icons` is present. |
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
portable = false
```

| Key | Default | Effect |
| --- | --- | --- |
| `categories` | `["Utility"]` | Fallback desktop category when `add` receives no categories and AppImage metadata has none. CLI `--categories` and extracted metadata take precedence. |
| `auto_extract` | `true` | Extract embedded AppImage `.desktop` metadata and icons during `add`. Set to `false` to skip extraction. |
| `update_desktop_db` | `true` | Silently invokes `update-desktop-database` after add, update, or remove when the utility is installed. |
| `dest_bin` | `"~/.local/bin"` | Default destination for terminal symlinks. `add --dest-bin` takes precedence; `~` expands to the configured home directory. |
| `symlink_appimages` | `false` | Controls PATH symlinks for AppImages only. The default skips them; set `true` to create them. The alias `appimage_symlinks` is also accepted. `--no-symlink` always wins. |
| `portable` | `false` | Automatically creates or updates `.musu.toml` portable bundle metadata file in the application's directory during `add` or `update`. |

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

Sandboxing is configured per **app type** through profiles: each
`[sandbox.<type>]` table holds the enablement flag, the backend, and the
restrictions applied to every sandboxed app of that type. Today the only
supported type is `appimage` — a profile section with any other name is
reported by `musu doctor` ("unknown sandbox profile") and ignored by
resolution.

```toml
[sandbox.appimage]
enabled = false
backend = "auto"
network = true
home_writable = true
hide_paths = []
extra_rw = []
extra_ro = []
```

| Key | Default | Effect |
| --- | --- | --- |
| `enabled` | *unset* | When `true`, newly added or updated **AppImages** without an explicit sandbox override get a sandbox wrapper in their `.desktop` file and symlink. Scripts and plain binaries have no profile and are never wrapped implicitly — pass `--sandbox` explicitly to sandbox them. A profile without the `enabled` key is treated as off and reported by `musu doctor`. |
| `backend` | `"auto"` | Selects the sandbox backend to use for this type: `"auto"` probes for an installed `bwrap` (Bubblewrap) first, falls back to `firejail`, and resolves to `"none"` (apps keep running unsandboxed) when neither is available. `"bwrap"`, `"firejail"`, and `"none"` force a specific backend. |
| `network` | `true` | When `false`, the sandbox gets no network access (`bwrap`: `--unshare-all` without `--share-net`; `firejail`: `--net=none`). |
| `home_writable` | `true` | When `false`, `$HOME` is mounted read-only inside the sandbox (`bwrap`: `--ro-bind`). Firejail cannot enforce this and ignores it with a warning. |
| `hide_paths` | `[]` | Paths hidden entirely inside the sandbox (`bwrap`: empty `tmpfs` mounted on top; `firejail`: `--blacklist`). Accepts `~`. |
| `extra_rw` | `[]` | Additional paths exposed read-write (`bwrap`: `--bind`). Firejail cannot enforce this and ignores it with a warning. Accepts `~`. |
| `extra_ro` | `[]` | Additional paths exposed read-only (`bwrap`: `--ro-bind`; `firejail`: best-effort `--blacklist`). Accepts `~`. |

`~/.ssh` and `~/.gnupg` are **always hidden** inside the sandbox. No policy can expose them: they are masked with an empty `tmpfs` (`bwrap`) or blacklisted (`firejail`) even when a `hide_paths`, `extra_rw`, or `extra_ro` entry points at them.

The generated wrapper always exports `APPIMAGE_EXTRACT_AND_RUN=1` (`bwrap`: `--setenv`; `firejail`: `--env=`). FUSE is not available inside the sandbox — `/dev/fuse` does not exist in bwrap's minimal `/dev` and the setuid `fusermount` helper does not resolve in user namespaces — so the AppImage runtime extracts itself to a temporary directory and runs from there on every launch (a bit slower on the first start) instead of mounting via FUSE. Non-AppImage targets simply ignore the variable.

### Enablement layers

The most explicit layer wins:

1. The per-app `sandbox` flag in `apps.toml` (`--sandbox` / `--no-sandbox`) wraps or unwraps the app no matter what the profile says — `--sandbox` works even for a type with no profile, sandboxing it with the defaults (`backend = "auto"`, permissive policy).
2. Otherwise, the profile of the app's type applies when `enabled = true` (only AppImages resolve to a profile today).
3. Otherwise the app runs unsandboxed.

### Migrating from the flat `[sandbox]` format

Profiles replaced the old flat layout, where `[sandbox]` held `enabled` and
`backend` directly and the restrictions lived in a nested `[sandbox.policy]`
table. That format is **rejected**: loading it fails with a hint instead of
being silently reinterpreted. Rewrite it as a per-type profile:

```toml
# Before (legacy)                     # After
[sandbox]                             [sandbox.appimage]
enabled = true                        enabled = true
backend = "auto"                      backend = "auto"

[sandbox.policy]                      # (policy keys now live directly in
network = false                       #  the profile, without this level)
```

### Per-app overrides (`apps.toml`)

Each entry in `~/.config/musu/apps.toml` may carry its own `sandbox` on/off flag and a `sandbox_policy` table stored next to it:

```toml
[[apps]]
id = "firefox"
name = "Firefox"
target_bin = "/home/you/Apps/Firefox.AppImage"
sandbox = true

[apps.sandbox_policy]
network = true
hide_paths = ["~/Secretos"]
```

The per-app policy is merged over the app type's profile **field by field**:

- A field present in the app replaces the profile value; an absent field inherits it.
- List fields (`hide_paths`, `extra_rw`, `extra_ro`) **replace** the profile list entirely — they are never concatenated.
- Fields unset everywhere fall back to the effective defaults: `network = true`, `home_writable = true`, and empty path lists.

### Example

```toml
[sandbox.appimage]
enabled = true
backend = "auto"
network = false        # AppImages get no network access by default
home_writable = false  # $HOME is mounted read-only
hide_paths = ["~/Private", "~/Work/clients"]
extra_ro = ["/srv/shared-assets"]
extra_rw = ["~/Work/active"]
```

Individual apps can opt back in (or out) per field with CLI flags on `musu add`/`musu update` — see [commands.md](commands.md#sandbox-policy-flags).

Editing a profile does not rewrite existing launchers on its own. Run `musu doctor --fix` to regenerate every registered AppImage whose baked-in `Exec` no longer matches the current profile (per-app overrides keep winning over the profile values). Non-AppImage apps must be updated manually with `musu update <app>`.
