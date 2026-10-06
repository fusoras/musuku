# Geico CLI Reference

Complete reference for the `geico` command-line interface.

```text
Usage: geico <COMMAND>

Commands:
  add      Register an executable and integrate it into the desktop and PATH
  list     List all managed applications with health checks
  update   Update metadata, target executable, or icon of a registered application
  remove   Safely remove desktop launcher and symlink (preserves original executable)
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help   Print help
```

Every subcommand supports `-h, --help` for its own usage text.

## Conventions used in this document

- **`$HOME`** — your home directory (e.g. `/home/you`).
- **id** — the slugified application identifier derived from the display name
  (`My Cool App!` → `my-cool-app`). Used for the `.desktop` filename, the default
  symlink name, and as a lookup key.
- **lookup** — `update` and `remove` accept either the *id* or the *display
  name*, matched **case-insensitively** (first match wins).

---

## `geico add`

Register an executable and integrate it into the desktop and PATH.

```text
Usage: geico add [OPTIONS] <EXECUTABLE>
```

### Arguments

| Argument       | Required | Description                                                                                                     |
| -------------- | -------- | --------------------------------------------------------------------------------------------------------------- |
| `<EXECUTABLE>` | yes      | Path to the executable file (AppImage, binary, or script). It must exist, be a regular file, and be executable. |

### Options

| Option                | Argument          | Default                     | Description                                                                                       |
| --------------------- | ----------------- | --------------------------- | ------------------------------------------------------------------------------------------------- |
| `--name`              | `<NAME>`          | executable filename         | Display name of the application. The id is the slug of this name.                                 |
| `--icon`              | `<PATH>`          | none                        | Icon file. **Copied** to `~/.local/share/icons/`; the source file is not moved or modified.        |
| `--categories`        | `<CATEGORIES>`    | none                        | Desktop categories **separated by commas**, e.g. `Development,Utility` (written as `Development;Utility;`). |
| `--bin-name`          | `<NAME>`          | the application id          | Custom command name for the terminal symlink.                                                     |
| `--no-desktop`        | flag              | desktop entry created       | Skip creating a `.desktop` launcher entry.                                                        |
| `--no-symlink`        | flag              | symlink created             | Skip creating a terminal `$PATH` symlink.                                                         |
| `--dest-bin`          | `<PATH>`          | `~/.local/bin`              | Custom directory for the symlink. Prompts for `sudo` only if the directory is outside `$HOME` and not writable. |
| `-h, --help`          | —                 | —                           | Print help.                                                                                       |

### Name and id fallback

- Without `--name`, the display name is the **final path component** of the
  executable, excluding a trailing `.appimage` extension
  case-insensitively:
  - `/opt/blender-4.2/blender` → name `blender`, id `blender`
  - `~/Downloads/Firefox.AppImage` → name `Firefox`, id `firefox`
- The **id** is always the slug of the display name: ASCII lowercase, every run
  of non-alphanumeric characters collapsed into a single `-`, with no leading or
  trailing `-`. Example: `Foo Bar 2.0` → `foo-bar-2-0`.
- The id determines:
  - the launcher filename: `~/.local/share/applications/<id>.desktop`
  - the default symlink name: `~/.local/bin/<id>` (unless `--bin-name` is given)
  - how the app can be referenced by `update`/`remove`

### What `add` creates

| Artifact         | Location                                  | Skipped with    |
| ---------------- | ----------------------------------------- | --------------- |
| Desktop launcher | `~/.local/share/applications/<id>.desktop` | `--no-desktop`  |
| Icon copy        | `~/.local/share/icons/<icon filename>`     | (only with `--icon`) |
| Terminal symlink | `~/.local/bin/<bin-name or id>` → target   | `--no-symlink`  |
| Registry entry   | `~/.config/geico/apps.toml`                | never           |

### Examples

```sh
# AppImage with a desktop launcher, custom icon, name, and categories
geico add ~/Downloads/cursor.AppImage \
  --name "Cursor" \
  --icon ~/Pictures/cursor.png \
  --categories Development

# Binary extracted from tar.xz: terminal PATH only, no desktop file
geico add /opt/tools/my-cli --no-desktop

# GUI app: desktop launcher + icon, but no terminal symlink
geico add /opt/blender/blender --no-symlink --icon /opt/blender/icon.png

# Custom command name and a symlink directory outside $HOME (sudo is prompted)
geico add /opt/tools/my-cli --bin-name mycli --dest-bin /usr/local/bin
```

### Troubleshooting

| Symptom / message                                | Cause & fix                                                                                                     |
| ------------------------------------------------ | --------------------------------------------------------------------------------------------------------------- |
| `target does not exist: …`                       | Wrong path — pass the real file location (check with `ls`).                                                      |
| `target is not a file: …`                        | A directory was passed; point at the file itself.                                                                |
| `target is not executable: …`                    | The file lacks the execute bit — `chmod +x path/to/AppImage` (typical for freshly extracted archives/AppImages). |
| `destination is not writable: …`                 | The symlink directory is inside `$HOME` but not writable. Fix permissions (`chmod u+w dir`) or choose another `--dest-bin`. `geico` never uses `sudo` inside `$HOME`. |
| `failed to invoke sudo` / `sudo command failed`  | `--dest-bin` points outside `$HOME` and is unwritable; approve the prompt or pick a writable directory.          |
| Launcher appears with a generic icon            | No `--icon` was given — add one with `geico update <app> --icon <path>`.                                         |
| Menu entry does not show up immediately         | Desktop environments cache menus; log out/in or wait for the file watcher to notice `~/.local/share/applications`. |
| Two apps share an id (second launcher overwrites the first) | `add` does not deduplicate ids — give distinct `--name` values, or `geico remove` the old entry first.  |

---

## `geico list`

List all managed applications with health checks.

```text
Usage: geico list
```

Takes no arguments or options besides `-h, --help`.

### Output format

```text
<name> (<id>) target=<status> desktop=<status> symlink=<status>
```

| Status           | Color        | Meaning                                                             |
| ---------------- | ------------ | ------------------------------------------------------------------- |
| `ok`             | bold green   | The path exists.                                                    |
| `missing/broken` | bold red     | The recorded path no longer exists (binary moved/deleted, link gone). |
| `disabled`       | bold yellow  | The artifact was intentionally not created (`--no-desktop`/`--no-symlink`). |

### Example

```console
$ geico list
Cursor (cursor) target=ok desktop=ok symlink=ok
My Tool (my-tool) target=ok desktop=ok symlink=ok
jq (jq) target=ok desktop=disabled symlink=ok
```

### Troubleshooting

| Symptom                         | Cause & fix                                                                                                 |
| ------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| `target=missing/broken`         | The original executable moved or was deleted. Re-point it: `geico update <app> --target <new-path>`.         |
| `desktop=missing/broken`        | The `.desktop` file was deleted manually. `geico update <app> --name <name>` regenerates it, or re-add.       |
| `symlink=missing/broken`        | The symlink was removed (e.g. by cleaning `~/.local/bin`). Re-create it via `geico update <app> --target <same-path>` or re-add. |
| Empty output                    | Nothing registered yet — run `geico add <executable>`.                                                       |

---

## `geico update`

Update metadata, target executable, or icon of a registered application.

```text
Usage: geico update [OPTIONS] <ID_OR_NAME>
```

### Arguments

| Argument        | Required | Description                                                       |
| --------------- | -------- | ----------------------------------------------------------------- |
| `<ID_OR_NAME>`  | yes      | ID or name of the registered application (case-insensitive lookup). |

### Options

| Option            | Argument      | Default            | Description                                                                     |
| ----------------- | ------------- | ------------------ | ------------------------------------------------------------------------------- |
| `--target`        | `<PATH>`      | unchanged          | New path to the executable. Validated exactly like `add` (exists, file, executable). |
| `--icon`          | `<PATH>`      | unchanged          | New icon file. The previously **installed icon copy is deleted** first, then the new one is copied. |
| `--name`          | `<NAME>`      | unchanged          | New display name. The id is re-derived (slug) from it, which also renames the `.desktop` file. |
| `-h, --help`      | —             | —                  | Print help.                                                                     |

### Behavior

- The entry is found by id or name; otherwise: `application not found: <query>`.
- If the app has a `.desktop` file, it is **regenerated** with the new
  name/id/target/icon.
- If the app has a terminal symlink, it is removed and re-created pointing at the
  new target, keeping the same link filename.
- Passing `--icon` to an app that has no desktop entry simply clears any recorded
  icon (icons are only referenced from the launcher).
- The registry is saved to `apps.toml` after the change.

### Examples

```sh
# point an app at a new AppImage release
geico update cursor --target ~/Downloads/cursor-v2.AppImage

# swap the icon (old icon copy is removed automatically)
geico update cursor --icon ~/Pictures/cursor-new.png

# rename an app: display name, id, and .desktop filename all change
geico update "My Tool" --name "My Tool Pro"

# lookups are case-insensitive, by id or by name
geico update CURSOR --target /opt/cursor/cursor
```

### Troubleshooting

| Symptom / message                     | Cause & fix                                                                                                             |
| ------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `application not found: …`            | The query matches no id/name — check `geico list` and use the exact id or name.                                          |
| `target does not exist/is not executable` | Same validation as `add`: verify the new path and its execute bit.                                                  |
| Icon did not change                   | The app was added with `--no-desktop`; icons are only meaningful for entries that have a launcher.                       |
| Symlink moved back to `~/.local/bin`  | `update` re-creates the link in the default bin directory; if you originally used a custom `--dest-bin`, move/re-create the link there manually or re-add the app. |
| Rename left a stale `<old-id>.desktop`| Normally regenerated/cleaned automatically; if it persists (e.g. manual edits), delete it from `~/.local/share/applications`. |

---

## `geico remove`

Safely remove desktop launcher and symlink (preserves original executable).

```text
Usage: geico remove <ID_OR_NAME>
```

### Arguments

| Argument       | Required | Description                                                       |
| -------------- | -------- | ----------------------------------------------------------------- |
| `<ID_OR_NAME>` | yes      | ID or name of the registered application (case-insensitive lookup). |

### What it does

1. Removes the generated `~/.local/share/applications/<id>.desktop` (if any).
2. Removes the installed icon copy (if any).
3. Removes the terminal symlink (if any), using the same sudo policy as `add`.
4. Drops the entry from `~/.config/geico/apps.toml`.

**The original executable/AppImage/script (`target_bin`) is NEVER deleted.**
There are no options besides `-h, --help`.

### Examples

```sh
geico remove cursor        # by id
geico remove "Cursor"      # by display name
geico remove CURSOR        # lookup is case-insensitive
```

### Troubleshooting

| Symptom / message                        | Cause & fix                                                                                                       |
| ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `application not found: …`               | Nothing matches the query — run `geico list` to see registered ids/names.                                          |
| `destination is not writable: …` (on remove) | The symlink lives in a non-writable directory inside `$HOME`; fix permissions manually and delete the link.    |
| `sudo` prompt during remove              | The symlink lives outside `$HOME` in an unwritable directory (only reachable via a previous `--dest-bin`).         |
| Original binary still works afterwards   | Expected — geico only ever deletes artifacts it created.                                                           |

---

## Exit behavior and error format

Errors are printed to stderr with an `anyhow` context chain, e.g.:

```text
Error: target does not exist: /tmp/does-not-exist

Caused by:
    No such file or directory (os error 2)
```

Common global notes:

- `geico` has no configuration file other than the state database
  `~/.config/geico/apps.toml`; all behavior is controlled by flags.
- `sudo` is invoked **only** when a symlink destination is outside `$HOME` and
  not writable (see `--dest-bin`); never for desktop/icon/state operations.
- Paths recorded in `apps.toml` are absolute — moving the original executable
  breaks the links until you run `geico update <app> --target <new-path>`.
