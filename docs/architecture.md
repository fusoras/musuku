# Geico Architecture

This document describes the internal design of `geico`: how it integrates the
FreeDesktop/XDG specifications, where every artifact lives, when and why it
escalates to `sudo`, and the exact schema of its state file.

## High-level overview

```text
                 ┌───────────────────────────────────────────────┐
                 │                  geico CLI                    │
                 │   src/cli.rs (clap derive) → src/main.rs      │
                 └──────────────────────┬────────────────────────┘
                                        │
                              ┌─────────▼─────────┐
                              │  Manager (src/    │
                              │  manager.rs)      │
                              │  add/list/update/ │
                              │  remove           │
                              └─┬───────┬───────┬─┘
                                │       │       │
              ┌─────────────────▼─┐ ┌───▼─────┐ ┌▼──────────────────┐
              │ desktop.rs        │ │ config  │ │ symlink.rs        │
              │ FreeDesktop       │ │ .rs     │ │ $PATH symlink +   │
              │ .desktop + icon   │ │ apps    │ │ selective sudo    │
              │ generation        │ │ .toml   │ │                   │
              └───────────────────┘ └───┬─────┘ └───────────────────┘
                                        │
                              ┌─────────▼─────────┐
                              │ models.rs         │
                              │ AppEntry /        │
                              │ AppRegistry       │
                              └───────────────────┘
```

### Module responsibilities

| Module           | Responsibility                                                                 |
| ---------------- | ------------------------------------------------------------------------------ |
| `src/cli.rs`     | Command-line surface defined with `clap` derive (`Cli`, `Command`, arg structs). |
| `src/main.rs`    | Parses arguments, builds a `Manager` with the default paths, dispatches.        |
| `src/manager.rs` | High-level lifecycle orchestration for `add`, `list`, `update`, `remove`, plus target validation and health-status formatting. |
| `src/models.rs`  | Core data model: `AppEntry`, `AppRegistry`, `fallback_name`, `slugify`.         |
| `src/config.rs`  | Persistence of the registry to `apps.toml` (load/save, first-run semantics).    |
| `src/desktop.rs` | FreeDesktop `.desktop` entry generation, icon copying, cleanup.                  |
| `src/symlink.rs` | Terminal `$PATH` symlink installation/removal and the sudo escalation policy.    |
| `src/colors.rs`  | ANSI escape-code constants used by `list` output.                               |

The library crate (`src/lib.rs`) exposes all of these modules so the test suite
can drive `Manager` against isolated `tempfile` workspaces without ever touching
the real user configuration.

## FreeDesktop & XDG integration

`geico` follows two sets of freedesktop.org standards:

1. **XDG Base Directory Specification** — where user data/config files live.
2. **Desktop Entry Specification** — the format of `.desktop` launcher files.

### Directory resolution

Paths are resolved at startup (via the `dirs` crate, which implements the XDG
rules) and injected into the `Manager`:

| Purpose                | Resolution order                                                    | Typical value               |
| ---------------------- | ------------------------------------------------------------------- | --------------------------- |
| State database         | `$XDG_CONFIG_HOME/geico`, fallback `~/.config/geico`                | `~/.config/geico/apps.toml` |
| Desktop entries & icons| `$XDG_DATA_HOME/applications`, `$XDG_DATA_HOME/icons`; fallback `~/.local/share/...` | `~/.local/share/applications`, `~/.local/share/icons` |
| Terminal symlinks      | `~/.local/bin` (fixed user bin dir; not an XDG variable)            | `~/.local/bin`              |

`~/.local/share/applications` is the standard per-user applications directory
recognized by every FreeDesktop-compliant desktop environment, so launchers
appear in the application menu without registering anything extra. Icons are
**copied** (never moved) into `~/.local/share/icons/`, the user icon-theme
location.

### Generated `.desktop` file

A minimal, spec-conformant entry is written to
`~/.local/share/applications/<id>.desktop`:

```ini
[Desktop Entry]
Type=Application
Name=My Tool
Exec=/opt/tools/my-tool
Icon=/home/user/.local/share/icons/my-tool.png
Categories=Development;Utility;
Terminal=false
```

Design details:

- **`Exec`** is the *absolute* path to the original executable. If the path
  contains spaces, tabs, newlines, or quotes, it is single-quoted with POSIX
  escaping (`'` → `'\''`), as required by the spec's quoting rules.
- **`Icon`** points at the absolute path of the *installed copy* of the icon, so
  the launcher keeps working even if the original icon file is deleted. The key
  is omitted when no `--icon` was given.
- **`Categories`** is only written when `--categories` is provided; values are
  joined with `;` and given the mandatory trailing separator
  (`Development,Utility` → `Categories=Development;Utility;`).
- **`Terminal=false`** is always set (desktop launchers are GUI-style entries;
  CLI access is provided through the `$PATH` symlink instead).
- The file is named after the application **id** (slug), so renaming via
  `geico update --name` produces a correspondingly renamed launcher.

## Selective sudo escalation

Everything `geico` does is user-space by default. Privilege escalation is a
narrow, explicit exception implemented in `src/symlink.rs` and applies **only**
to symlink creation/removal:

```text
install link in DEST:
  if DEST is writable by the user     → std::fs::symlink (no sudo)
  else if DEST is outside $HOME       → sudo ln -sf TARGET LINK
  else                                → error "destination is not writable"
```

The same three-way decision guards removal (`sudo rm -f LINK` for unwritable
directories outside `$HOME`).

Rules encoded in this policy:

1. **Inside `$HOME`, sudo is never invoked.** A non-writable directory under your
   home is treated as a configuration problem and reported as an error.
2. **Outside `$HOME`, sudo runs only when the directory is actually unwritable.**
   A world-writable destination (e.g. a directory you own in `/opt`) is still
   handled without privileges.
3. **Escalation is opt-in.** The only way to reach an out-of-home destination is
   to pass `--dest-bin` explicitly; no default path lies outside `$HOME`.

Desktop entries and icons always live under the user's data directory and are
written with plain filesystem calls — they never trigger `sudo`.

## The `apps.toml` schema

State lives at `~/.config/geico/apps.toml` and is a direct TOML serialization of
the `AppRegistry` model (`src/models.rs`) via `serde` + `toml`.

### Example

```toml
[[apps]]
id = "my-tool"
name = "My Tool"
target_bin = "/home/user/tools/my-tool"
desktop_file = "/home/user/.local/share/applications/my-tool.desktop"
symlink = "/home/user/.local/bin/my-tool"
icon_path = "/home/user/.local/share/icons/my-tool.png"
categories = [
    "Development",
    "Utility",
]
created_at = "2026-10-06T01:00:10.302344024Z"

[[apps]]
id = "plain"
name = "plain"
target_bin = "/home/user/tools/plain"
symlink = "/home/user/.local/bin/plain"
created_at = "2026-10-06T01:00:10.345958959Z"
```

### Fields

| Field           | Type                      | Meaning                                                                                     |
| --------------- | ------------------------- | ------------------------------------------------------------------------------------------- |
| `id`            | string                    | Slugified display name (`My Cool App!` → `my-cool-app`). Key for `.desktop` filename and default symlink name. |
| `name`          | string                    | Human-readable display name; falls back to the executable filename when `--name` is absent.  |
| `target_bin`    | path string               | Original executable/AppImage/script. **Never modified or deleted by geico.**                 |
| `desktop_file`  | path string *(optional)*  | Generated `.desktop` launcher. Absent when added with `--no-desktop`.                        |
| `symlink`       | path string *(optional)*  | Generated `$PATH` symlink. Absent when added with `--no-symlink`.                            |
| `icon_path`     | path string *(optional)*  | Installed copy of the icon (source file is untouched).                                       |
| `categories`    | array of strings *(optional)* | Desktop categories as passed to `--categories`.                                          |
| `created_at`    | RFC 3339 UTC timestamp    | Registration time (nanosecond precision, `Z` suffix).                                        |

Optional fields are simply omitted from the file when unset (TOML has no `null`).

### Load/save semantics

- **Missing file** = first run → an empty registry is returned; not an error.
- **Malformed TOML** = hard error (`failed to parse config file …`); geico never
  silently discards state, which prevents data loss on a typo.
- **Saving** creates missing parent directories (`~/.config/geico/`) and writes
  pretty-printed TOML.
- Every mutation (`add`, `update`, `remove`) loads the registry, applies the
  change in memory, and saves atomically-enough for a single-user CLI (full-file
  rewrite).

## Application lookup

`update` and `remove` accept an `ID_OR_NAME` that matches an entry **case
-insensitively** against either the `id` or the `name`
(`id.eq_ignore_ascii_case(query) || name.eq_ignore_ascii_case(query)`). The first
match in registration order wins.

## Lifecycle flows

### `add`

1. Validate the target: must exist, be a regular file, and have at least one
   execute bit (`mode & 0o111`).
2. Build an `AppEntry` (explicit `--name` or filename fallback; slug the id).
3. Unless `--no-desktop`: copy the icon (if any) to `~/.local/share/icons/` and
   write `~/.local/share/applications/<id>.desktop`.
4. Unless `--no-symlink`: create `<dest>/<bin-name or id>` → target, replacing a
   stale link at that path; `--dest-bin` defaults to `~/.local/bin`.
5. Append the entry to the registry and save `apps.toml`.

### `list`

Load the registry and print one line per entry:
`Name (id) target=… desktop=… symlink=…`, where each status is `ok` (green),
`missing/broken` (red, recorded path no longer exists), or `disabled` (yellow,
artifact intentionally not created).

### `update`

1. Find the entry by id/name (error: `application not found: …`).
2. Apply `--target` (validated), `--name` (re-slugs the id), `--icon` (drops the
   previously installed icon copy).
3. If a `.desktop` file exists, regenerate it (new id/name/target/icon take
   effect, possibly under a new filename).
4. If a symlink exists, remove it and re-create it pointing at the new target,
   keeping the original link filename.
5. Persist the updated entry.

### `remove`

1. Find and extract the entry from the registry (error if unknown).
2. Delete the `.desktop` file and the installed icon copy.
3. Delete the symlink (with the same sudo policy as installation).
4. Save the registry.

The `target_bin` file is never part of any deletion step — this is the
non-destructive invariant, asserted by
`test_remove_deletes_generated_files_but_preserves_original_binary` and
`test_model_app_registry_remove_returns_entry_is_non_destructive`.

## Error handling & safety

- `anyhow` is used throughout: errors carry context chains such as
  `failed to write config file … / No such file or directory`.
- `unsafe_code = "forbid"` is set in `Cargo.toml`; Clippy's `all` group is
  enabled as warnings.
- The release profile targets small binaries: `opt-level = "z"`, LTO,
  `codegen-units = 1`, and `strip = true`.

## Testing strategy

Integration tests (`tests/`) run every filesystem operation inside
`tempfile::tempdir()` workspaces by passing explicit paths to
`Manager::new(config_path, data_dir, bin_dir)`. The default-path functions are
thin wrappers around the same logic, so tests exercise production code paths
without ever reading or writing the real `~/.config/geico/apps.toml`.
