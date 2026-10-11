# Musu Commands Reference

Complete command-line reference, options, flags, and advanced examples for `musu`.

```text
Usage: musu <COMMAND>

Commands:
  add          Register an executable and integrate it into desktop and PATH
  list         List all managed applications with health checks
  show         Show detailed information and paths for a registered application
  update       Update metadata, target executable, icon, categories, or description
  remove       Safely remove desktop launcher and symlink (preserves original executable)
  watch        Watch directories for AppImages to auto-register, update, or prune
  health       Check system environment, SVG converters, and desktop integration health
  run          Launch a registered application in the background
  completions  Generate or install shell completion scripts
```

Every command supports `-h, --help` for instant usage text.

---

## 1. `musu add`

Registers an executable file and integrates it into the desktop environment and terminal `$PATH`.

```sh
musu add [OPTIONS] <EXECUTABLE>
```

### Options

| Option | Description | Default |
| --- | --- | --- |
| `<EXECUTABLE>` | Path to the AppImage, binary, or script (must exist, be a regular file, and be executable). | *required* |
| `--name <NAME>` | Display name shown in desktop application menus. | Derived from executable filename |
| `--icon <PATH>` | Path to the icon image (copied to `~/.local/share/icons/`). | Extracted from AppImage or None |
| `--categories <LIST>` | Desktop categories separated by commas (e.g. `Development,Utility`). Use without a value to list standard categories (read-only query). | Extracted or configured defaults |
| `--description <TEXT>` | Application description or tooltip written to `Comment=` in the `.desktop` file (short: `-d`; visible aliases: `--comment`, `--desc`). Repeat the flag to add per-language descriptions using `LANG=TEXT` (e.g. `--description "Default" --description es="Texto en español"`); each language is written as `Comment[es]=...` so desktop environments show the matching locale. | Extracted from AppImage or None |
| `--bin-name <NAME>` | Custom command name for the terminal symlink. | Application ID (slug) |
| `--no-desktop` | Skip creating the `.desktop` launcher entry. | `false` |
| `--no-symlink` | Skip creating the terminal `$PATH` symlink. | `false` |
| `--dest-bin <PATH>` | Custom directory for the binary symlink (prompts for `sudo` if outside `$HOME`). | `~/.local/bin` |
| `--overwrite` | Replace an existing application with the same name/ID without asking (aliases: `--force`, `-f`). | `false` |
| `--sandbox` | Wrap the launcher and terminal symlink in a sandbox (bwrap or firejail; visible alias: `--isolate`). Required for scripts and plain binaries — the `[sandbox.appimage]` profile only wraps AppImages. | `[sandbox.appimage].enabled` |
| `--no-sandbox` | Keep this application unsandboxed, even when `[sandbox.appimage] enabled = true`. | `[sandbox.appimage].enabled` |
| `--network <true\|false>` | Enable or disable network access for this app, overriding the profile `network` in either direction. | profile `[sandbox.<type>]` |
| `--home-writable <true\|false>` | Mount `$HOME` writable or read-only for this app, overriding the profile `home_writable` in either direction. | profile `[sandbox.<type>]` |
| `--hide-path <PATH>` | Hide a path inside the sandbox (repeatable). Replaces the profile `hide_paths` for this app. | profile `[sandbox.<type>]` |
| `--extra-rw <PATH>` | Expose an extra path read-write in the sandbox (repeatable). | profile `[sandbox.<type>]` |
| `--extra-ro <PATH>` | Expose an extra path read-only in the sandbox (repeatable). | profile `[sandbox.<type>]` |
| `--portable` | Save metadata to a portable `.musu.toml` bundle file in the application's directory. | `false` |

### Sandbox Policy Flags

The policy flags store a per-app `sandbox_policy` in `apps.toml`, merged over the app type's profile (`[sandbox.<type>]`) field by field (see [config.md](config.md#sandbox)). The wrapper is only generated when the app is effectively sandboxed: the explicit `--sandbox` flag (which wraps any executable), or `enabled = true` in the profile of the app's type (today: `[sandbox.appimage]` for AppImages) — profiles never wrap scripts or plain binaries.

- Overrides are **bidirectional**: with a profile `network = false`, `--network true` exempts one app; with the default permissive policy, `--network false` restricts just this app. The same applies to `--home-writable`.
- `--hide-path`, `--extra-rw`, and `--extra-ro` are repeatable and **replace** the corresponding profile list for this app — they are never appended to it.
- `~/.ssh` and `~/.gnupg` are always hidden; no flag can expose them.

> [!WARNING]
> The `firejail` backend cannot reliably enforce `home_writable = false` and `extra_rw`. When `firejail` is the resolved backend, `musu add`/`musu update` print advisory warnings to stderr (never fatal; stdout stays clean) and ignore those restrictions. `extra_ro` is best-effort (emitted as `--blacklist`). The `bwrap` backend supports the full policy.

### Fallback Name & ID Behavior

- If `--name` is omitted, the display name is derived from the executable filename, stripping the `.AppImage` extension case-insensitively (e.g. `Firefox.AppImage` → `Firefox`, `/opt/blender-4.2/blender` → `blender`).
- The **ID** is always the slugified display name: ASCII lowercase, with consecutive non-alphanumeric characters replaced by hyphens (e.g. `My Cool App!` → `my-cool-app`).
- The ID determines the `.desktop` filename (`my-cool-app.desktop`), the default symlink name, and the lookup key for commands like `update`, `show`, `run`, and `remove`.

### AppImage Extraction & Symlink Defaults

- For Type 2 AppImages, when `--name`, `--icon`, `--categories`, or `--description` are omitted, `musu` inspects the embedded `.desktop` file and icon via `--appimage-extract`.
- By default, AppImages do **not** receive terminal symlinks to prevent polluting terminal `$PATH` with graphical apps, while standalone binaries and scripts always receive them.
- To enable terminal symlinks for AppImages by default, configure `~/.config/musu/config.toml`:

```toml
[defaults]
symlink_appimages = true
```

The alias `appimage_symlinks = true` is also accepted. Explicit `--no-symlink` always overrides configuration.

### Name Collisions

When the derived ID (or an orphaned `.desktop` file) already exists, `musu add` does not silently overwrite anything:

- **Interactive terminal**: shows a menu — `[r]` rename the new application, `[o]` overwrite the existing one, `[c]` cancel (default, also used for empty input).
- **Non-interactive** (piped stdin): aborts with an error unless `--overwrite`/`--force`/`-f` is passed.
- Renaming registers the application under the new name (e.g. `zen` → `zen-beta`), so multiple versions of the same app can coexist.
- Overwriting removes the previous `.desktop` launcher, symlink, and icon, but never deletes the original target executable.

### Examples

```sh
# Auto-detect icon, name, description, and register AppImage
musu add ~/Downloads/Obsidian.AppImage

# Full integration with explicit metadata
musu add ~/Downloads/myapp.AppImage \
  --name "My App" \
  --icon ~/Pictures/myapp.png \
  --categories AudioVideo,Utility \
  --description "Lightweight audio recorder"

# Multilingual descriptions: default plus per-language comments
musu add ~/Downloads/myapp.AppImage \
  --description "Lightweight audio recorder" \
  -d es="Grabadora de audio ligera" \
  -d fr="Enregistreur audio léger"

# Standalone CLI binary: symlink only, no desktop entry
musu add /opt/tools/my-cli --no-desktop --bin-name mycli

# Standalone GUI app: desktop launcher only, no terminal symlink
musu add /opt/blender/blender --no-symlink --icon /opt/blender/icon.png

# Sandbox an AppImage: no network access, one hidden folder
musu add ~/Apps/Firefox.AppImage --sandbox --network false --hide-path ~/Secretos

# Exempt a single app from a restrictive global policy (network = false)
musu add ~/Apps/Music.AppImage --sandbox --network true

# Grant an editor access to a workspace folder while hiding client files
musu add ~/Apps/Editor.AppImage --sandbox --extra-rw ~/Work/active --hide-path ~/Work/clients
```

---

## 2. `musu list`

Lists all managed applications with health checks, format options, and terminal icon previews.

```sh
musu list [OPTIONS]
```

### Options

| Option | Description |
| --- | --- |
| `--icons` | Display inline icon previews in compatible terminals using Kitty graphics protocol. |
| `--no-icons` | Suppress icon previews even in compatible terminals. |
| `-t, --table` | Format output as an aligned table with `ICON`, `NAME`, `ID`, `CATEGORIES`, and `STATUS` columns. |
| `--category <CAT>` | Filter applications by desktop category (case-insensitive substring/prefix match). |
| `--broken` | Show only applications whose target binary or artifacts are missing/broken. |
| `--hide-disabled-symlinks` | Hide symlink status indicator when intentionally disabled (`symlink=disabled`). Visible alias: `--hide-disabled`. |
| `--hide-target-ok` | Hide target status when healthy (`target=ok`). |
| `--hide-desktop-ok` | Hide desktop file status when healthy (`desktop=ok`). |
| `--hide-ok` | Hide both target and desktop statuses when healthy. Visible alias: `--hide-ok-status`. |
| `--description` | Display application descriptions in list output (aliases: `--descriptions`, `--desc`). |
| `--no-description` | Suppress application descriptions even if enabled in `config.toml` (aliases: `--no-descriptions`, `--no-desc`). |

### Status Indicators

- **`ok` (green)**: The path exists on disk and is healthy.
- **`missing/broken` (red)**: The target executable or generated artifact was moved or deleted.
- **`disabled` (yellow)**: The artifact was intentionally skipped (via `--no-desktop` or `--no-symlink`).

### Examples

```sh
# Default simple list
musu list

# List with inline terminal icon previews
musu list --icons

# List showing application descriptions
musu list --description

# Aligned table view with descriptions
musu list --table --description

# Filter only broken or missing applications
musu list --broken

# Hide healthy statuses for a cleaner compact view
musu list --hide-ok --hide-disabled
```

---

## 3. `musu show` (alias: `info`)

Inspects the metadata, artifact paths, description, and health status of a registered application.

```sh
musu show <ID_OR_NAME> [OPTIONS]
```

### Options

| Option | Description |
| --- | --- |
| `<ID_OR_NAME>` | Application ID or display name (case-insensitive). |
| `--bin` | Print only the target executable path. |
| `--desktop` | Print only the `.desktop` launcher path. |
| `--icon` | Print only the copied icon path. |
| `--symlink` | Print only the terminal symlink path. |
| `--json` | Print the complete application entry as formatted JSON. |
| `-a, --absolute` | Print full absolute paths instead of collapsing `$HOME` to `~` (alias: `--full-path`). |
| `--icons` | Display application icon preview in the terminal. |
| `--no-icons` | Suppress terminal image preview. |

### Output Format

```text
My App (my-app)
  Executable:  ~/Downloads/myapp [ok]
  Desktop:     ~/.local/share/applications/my-app.desktop [ok]
  Icon:        ~/.local/share/icons/my-app.png [ok]
  Symlink:     ~/.local/bin/my-app [ok]
  Categories:  AudioVideo, Utility
  Description: Lightweight audio recorder
    [es]: Grabadora de audio ligera
    [fr]: Enregistreur audio léger
  Registered:  2026-10-06 14:00:00 UTC
Sandbox: active (bwrap) [appimage]
  ✗ network    → no network access
  ✗ ~/.ssh     → hidden
  ✗ ~/.gnupg   → hidden
```

Per-language descriptions (registered via `-d LANG=TEXT`) are listed indented under `Description`.

The `Sandbox` block reports the effective state: `active (bwrap) [appimage]` / `active (firejail) [appimage]` (the bracket names the applied profile), `active (bwrap)` when the app is sandboxed by an explicit `--sandbox` flag without any profile behind it, or `inactive` for unsandboxed apps (including non-AppImages registered without an explicit `--sandbox`). Each line is a denied permission, showing the fully merged policy (the type's profile plus per-app overrides):

- `✗` (red): a restriction the backend enforces.
- `!` (yellow): a restriction the `firejail` backend cannot enforce and will ignore (e.g. `home_writable = false`, `extra_rw`).
- `~/.ssh` and `~/.gnupg` are always listed when the sandbox is active.

With `--json`, the computed view is injected under the `sandbox` key (the raw stored `sandbox_policy` also appears as a regular entry field):

```json
"sandbox": {
  "backend": "bwrap",
  "denied": [
    "~/.ssh hidden",
    "~/.gnupg hidden",
    "~/Secretos hidden by policy"
  ],
  "enabled": true,
  "profile": "appimage",
  "policy": {
    "extra_ro": [],
    "extra_rw": [],
    "hide_paths": ["~/Secretos"],
    "home_writable": true,
    "network": false
  }
}
```

`policy` is the effective policy with every field filled in; `denied` combines each denied label with its human-readable reason. `profile` is the type whose profile governs the app (`"appimage"`), or `null` when the sandbox is inactive or runs without a profile (e.g. a flag-forced script).

### Path Extraction for Scripts

```sh
# Open the generated desktop file directly in your editor
nano $(musu show my-app --desktop)

# Inspect the raw JSON entry
musu show my-app --json | jq .
```

---

## 4. `musu update`

Updates the metadata, target executable path, icon, categories, or description of a registered application without re-registering it.

```sh
musu update <ID_OR_NAME> [OPTIONS]
```

### Options

| Option | Description | Default |
| --- | --- | --- |
| `<ID_OR_NAME>` | ID or display name of the application to update. | *required* |
| `--target <PATH>` | New path to the target executable (validated like `add`). | target unchanged |
| `--icon <PATH>` | New icon file (removes previously copied icon first). | icon unchanged |
| `--name <NAME>` | New display name (re-derives slug ID and updates desktop entry). | name unchanged |
| `--categories <LIST>` | New desktop categories (e.g. `Graphics,3DGraphics`). Use without a value to show current categories (read-only query). | categories unchanged |
| `--description <TEXT>` | New description or tooltip for the application (short: `-d`; visible aliases: `--comment`, `--desc`). Repeat with `LANG=TEXT` to set per-language descriptions; each `LANG=text` replaces only that language (others are kept), and an empty value (`LANG=`) removes it. | description unchanged |
| `--sandbox` | Wrap the launcher and terminal symlink in a sandbox (visible alias: `--isolate`). The explicit flag is also the only way to wrap scripts and plain binaries — profiles alone only cover AppImages. | sandbox unchanged |
| `--no-sandbox` | Keep this application unsandboxed. | sandbox unchanged |
| `--network <true\|false>` | Enable or disable network access for this app, overriding the profile `network` in either direction. | inherited from the type's profile |
| `--home-writable <true\|false>` | Mount `$HOME` writable or read-only for this app, overriding the profile `home_writable` in either direction. | inherited from the type's profile |
| `--hide-path <PATH>` | Hide a path inside the sandbox (repeatable). Replaces the profile `hide_paths` for this app. | inherited from the type's profile |
| `--extra-rw <PATH>` | Expose an extra path read-write in the sandbox (repeatable). | inherited from the type's profile |
| `--extra-ro <PATH>` | Expose an extra path read-only in the sandbox (repeatable). | inherited from the type's profile |
| `--remove-symlink` | Delete the terminal symlink and clear `symlink` in registry. | symlink kept |
| `--portable` | Save or update `.musu.toml` bundle file in the application's directory. | `false` |

> [!NOTE]
> Modifying `--target`, `--name`, `--icon`, `--categories`, or `--description` regenerates the `.desktop` file and re-points the terminal symlink automatically when those artifacts exist.
> `--remove-symlink` deletes the symlink file without touching the desktop launcher, icon, or original binary.

> [!NOTE]
> Sandbox policy flags (`--network`, `--home-writable`, `--hide-path`, `--extra-rw`, `--extra-ro`) also count as changes: they regenerate the `.desktop` file so the launcher picks up the new wrapper. The app's stored `sandbox_policy` is **replaced** by the flags you pass — flags you omit revert to inheriting the type's profile policy. Passing only `--sandbox`/`--no-sandbox` without any other change is reported as *No changes* and does not regenerate the launcher; combine the flag with another change (e.g. a policy flag) to apply it. To drop a stored per-app policy entirely, delete its `sandbox_policy` table from `apps.toml` and re-register with `musu add --overwrite`.

### Examples

```sh
# Point application to a newly downloaded AppImage version
musu update my-app --target ~/Downloads/myapp-v2.AppImage

# Rename application
musu update my-app --name "My App Pro"

# Update application description
musu update my-app --description "New updated audio suite"

# Update only the Spanish description, keeping other languages
musu update my-app -d es="Nueva suite de audio"

# Remove the French description
musu update my-app -d fr=

# Update categories
musu update my-app --categories AudioVideo,Development

# Remove terminal symlink while keeping desktop launcher
musu update my-app --remove-symlink

# Restrict one app: no network access, launcher regenerated with the new policy
musu update my-app --network false

# Exempt an app from a restrictive global network policy
musu update my-app --network true

# Hide an extra folder and expose a shared data directory read-only
musu update my-app --hide-path ~/Secretos --extra-ro /srv/shared-assets
```

---

## 5. `musu remove`

Safely removes the generated `.desktop` launcher, copied icon, and terminal symlink, and drops the entry from `apps.toml`.

```sh
musu remove [ID_OR_NAME...] [OPTIONS]
```

### Options

| Option | Description | Default |
| --- | --- | --- |
| `[ID_OR_NAME...]` | One or more IDs or names of registered applications to remove. | *required unless `--all`* |
| `--all` | Remove all registered applications from the registry (prompts for confirmation). | `false` |
| `--delete-target` | Also delete the original executable/AppImage from disk (alias: `--delete-appimage`). Prompts for confirmation. | `false` |
| `-y, --yes` | Skip confirmation prompts (required for scripts/non-interactive stdin). | `false` |

> [!NOTE]
> **Safety Guarantee**: Without `--delete-target`, your original AppImage, binary file, or script is **never deleted**. Only the artifacts created by `musu` are removed.
> When `--delete-target` is used, `musu` asks for explicit confirmation before touching the executable on disk.

### Examples

```sh
# Remove single application by ID or name
musu remove my-app

# Remove multiple applications at once
musu remove app-one app-two app-three

# Remove all applications
musu remove --all -y

# Complete uninstall including deleting the original executable from disk
musu remove my-app --delete-target
```

---

## 6. `musu watch`

`musu watch` manages "live" AppImage directories. Rather than requiring a persistent, blocking terminal process, `musu` offers transparent, on-demand synchronization:
- **Clean on delete**: When an AppImage is deleted, its registry entry, `.desktop` launcher, and terminal symlink are automatically removed.
- **Autonomous version upgrades**: When an existing AppImage is replaced by or accompanied by a newer version (e.g. `sonora-v0.38.0-x86_64.AppImage` -> `sonora-v0.42.1-x86_64.AppImage`), `musu` autonomously updates the registration, retargets the `.desktop` launcher and terminal symlink, and re-extracts the application icon without duplicating entries.
- **Automatic detection on `musu list`**: Any configured watch directory is automatically synchronized in milliseconds every time `musu list`, `musu show`, or `musu run` is executed.
- **Fast one-shot sync**: Calling `musu watch [DIRECTORIES...]` synchronizes the folders immediately and exits. It automatically persists new directories to `config.toml` so future commands keep them synchronized.

```sh
musu watch [DIRECTORIES...] [OPTIONS]
```

### Options

| Option | Description | Default |
| --- | --- | --- |
| `[DIRECTORIES...]` | Directories to scan and monitor (accepts `.`, relative, or absolute paths). If omitted, uses `[watch].directories` from `config.toml`. | None |
| `--unwatch <DIR...>` | Remove one or more directories from configured watch directories (alias: `--remove-dir`). | None |
| `--continuous` | Keep running a debounced filesystem watcher in foreground (alias: `--foreground`). Default is one-shot sync and exit. | `false` |
| `--scan-only` | Run the scan once and exit without launching the continuous filesystem watcher (alias: `--once`). Default behavior. | `true` |
| `-r, --recursive` | Search and monitor subdirectories recursively. | `false` |
| `--icons-dir <DIR>` | Directory to search for matching icons (matched by stem/normalized application name: `png`, `svg`, `webp`, `xpm`, or `ico`). | None |
| `--dry-run` | Preview what would be added, updated, or removed without modifying the registry. | `false` |

### Configuration (`~/.config/musu/config.toml`)

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

### Examples

```sh
# Define and synchronize a directory as a live watch folder
musu watch ~/Applications

# Synchronize current directory
musu watch .

# Scan once without leaving a background watcher (default behavior)
musu watch ~/Downloads/ --scan-only

# Preview changes with dry-run
musu watch ~/Applications/ --dry-run

# Stop watching / remove a directory from watch configuration
musu watch --unwatch ~/Applications/

# Run persistent foreground watcher loop (optional)
musu watch ~/Applications --continuous
```

### Background Watch Service (systemd)

`install.sh` automatically generates the systemd user service unit in `~/.config/systemd/user/musu-watch.service`. Instead of keeping a terminal process running with `--continuous`, you can manage the native user service directly:

```sh
# Enable and start the background watcher service (no root needed)
systemctl --user daemon-reload
systemctl --user enable --now musu-watch.service

# Check live service status
systemctl --user status musu-watch.service

# View real-time logs
journalctl --user -u musu-watch.service -f

# Stop and disable service
systemctl --user stop musu-watch.service
systemctl --user disable --now musu-watch.service
```

---

## 7. `musu health`

Reports available SVG converters, terminal Kitty Graphics Protocol support, desktop utilities, environment paths, and healthy/broken registry entries.

```sh
musu health [OPTIONS]
```

### Options

| Option | Description |
| --- | --- |
| `--json` | Output the complete health report as machine-readable JSON. |
| `-v, --verbose` | List every individual tool in capability groups (missing tools shown as dim `[-]`). |

### SVG Converter Priority

SVG icon previews are rasterized with the first converter available in this priority order:
1. `rsvg-convert` (`librsvg2-bin`) — fastest startup, native GNOME rendering.
2. `magick` (ImageMagick 7) — general-purpose fallback.
3. `convert` (ImageMagick 6) — legacy fallback subject to `policy.xml`.

If none is installed, previews are skipped silently without errors and `.desktop` launchers continue to work normally.

### Examples

```sh
# Capability summary
musu health

# Detailed per-tool inventory
musu health --verbose

# JSON export
musu health --json
```

---

## 8. `musu doctor`

Audits the environment and application registry to detect missing directories, broken symlinks, missing executables, and orphaned desktop files. Optionally repairs these issues automatically.

```sh
musu doctor [OPTIONS]
```

### Options

| Option | Description |
| --- | --- |
| `--fix` | Automatically repair detected issues (prune broken symlinks, clean dead entries, regenerate outdated sandbox launchers for AppImages). |
| `-y, --yes` | Confirm `--fix` operations without interactive prompts. |

### Examples

```sh
# Run diagnostics
musu doctor

# Auto-repair issues and confirm all prompts
musu doctor --fix --yes
```

`--fix` also re-applies sandbox policy changes to existing apps: the sandbox wrapper is baked into two entry points — the launcher's `Exec=` line and the generated `$PATH` wrapper script — so editing the `[sandbox.appimage]` profile in `config.toml` leaves registered apps running the old command. `musu doctor` flags each **AppImage** whose `Exec` no longer matches the effective policy (the type's profile merged with any per-app override), and `musu doctor --fix` regenerates both the `.desktop` file and the `$PATH` wrapper for those apps, keeping name, icon, categories, and per-app policy overrides intact. Apps that are not AppImages (scripts, extracted binaries) are never detected or regenerated automatically; apply the new policy to them manually with `musu update <app>` or re-register with `musu add --overwrite`. Running `--fix` twice in a row is a no-op once every launcher matches.

`musu doctor` also warns about sandbox profiles that can never fire: a section for an unsupported type (`unknown sandbox profile 'xyz' (supported: appimage)`) and a supported profile without the `enabled` key (`sandbox profile 'appimage' has no 'enabled' key; assuming false`). Warnings are advisory — they never count as issues and never fail the run.

---

## 9. `musu run`

Launches a registered application in the background, detached from the current terminal, passing through any additional arguments.

```sh
musu run <ID_OR_NAME> [ARGS...]
```

### Examples

```sh
# Launch application in background
musu run obsidian

# Launch with pass-through flags
musu run obsidian --new-window ~/Notes/
```

---

## 10. Categories Query Mode (`--categories` without a value)

`--categories` accepts no value as a read-only query mode:
- `musu update <app> --categories`: prints the application's current categories followed by the standard FreeDesktop category table.
- `musu add <path> --categories`: prints only the standard FreeDesktop category table.

Values that do not match a standard category (or a known alias) are kept as custom categories, with an advisory `Did you mean:` warning on stderr.

```sh
# Query categories for an existing app
musu update my-app --categories

# View standard categories table
musu add ~/bin/tool --categories
```

---

## 11. `musu completions`

Generates shell completion scripts for Tab autocompletion.

```sh
musu completions [SHELL] [OPTIONS]
```

- Supported shells: `bash`, `zsh`, `fish`, `elvish`, `powershell` (auto-detected if omitted).

### Options

| Option | Description |
| --- | --- |
| `--install` | Automatically install completions into your user configuration. |
| `--stdout` | Print raw completion script to stdout. |

### Examples

```sh
# Install completions automatically for current shell
musu completions --install

# Export completions for Fish shell
musu completions fish --stdout > ~/.config/fish/completions/musu.fish
```
