# User Guide

`musu` integrates portable applications—AppImages, standalone binaries, extracted archives, and custom scripts—into your desktop application menu and terminal `$PATH` with a single command.

---

## Common Scenarios

### 1. Integrating an AppImage

Downloaded an AppImage like Obsidian, Krita, or Cursor?

```sh
# Basic integration
musu add ~/Downloads/Obsidian-1.5.8.AppImage

# With a custom icon and desktop categories
musu add ~/Downloads/Obsidian-1.5.8.AppImage \
  --name "Obsidian" \
  --icon ~/Pictures/obsidian.png \
  --categories "Office,Utility"
```

**What happens:**

- An application launcher is created at `~/.local/share/applications/obsidian.desktop`.
- The icon is copied to `~/.local/share/icons/obsidian.png`.
- A terminal shortcut is created at `~/.local/bin/obsidian`, so you can just type `obsidian` in any terminal.

---

### 2. Integrating an Extracted Archive (e.g., Blender, Postman)

Extracted a `.tar.xz` or `.zip` archive containing a standalone binary?

```sh
musu add /opt/blender-4.2/blender \
  --name "Blender" \
  --icon /opt/blender-4.2/blender.svg \
  --categories "Graphics,3DGraphics"
```

---

### 3. Integrating a Custom Shell Script

Have a personal script you want available globally in your terminal and launcher?

```sh
musu add ~/scripts/backup-notes.sh \
  --name "Backup Notes" \
  --bin-name "backup-notes" \
  --categories "Utility"
```

---

## Managing Your Applications

### Check App Details and Paths (`musu show`)

Inspect the exact file locations and health of any registered application:

```sh
musu show obsidian
```

Output:

```text
Obsidian (obsidian)
  Executable:  ~/Downloads/Obsidian-1.5.8.AppImage [ok]
  Desktop:     ~/.local/share/applications/obsidian.desktop [ok]
  Icon:        ~/.local/share/icons/obsidian.png [ok]
  Symlink:     ~/.local/bin/obsidian [ok]
  Categories:  Office, Utility
  Registered:  2026-10-06 14:00:00 UTC
Sandbox: inactive
```

By default, paths located within your home directory are shortened with `~/` for clean, readable output. To display the full absolute path including your username, pass `-a` or `--absolute`:

```sh
# Display full paths without ~/
musu show obsidian --absolute
```

You can also extract a single path for scripts or terminal shortcuts:

```sh
# Print only the executable path
musu show obsidian --bin

# Print only the .desktop launcher path
musu show obsidian --desktop

# Output as JSON
musu show obsidian --json
```

In terminals that support image previews (such as Kitty, Ghostty, WezTerm, and Herdr),
`musu show` displays the application icon inline above the card by default. Pass
`--no-icons` to keep the output as plain text, or `--icons` to force it:

```sh
# Force the text card, skipping any terminal image preview
musu show obsidian --no-icons

# Explicitly request icon preview
musu show obsidian --icons
```

---

### List All Apps & Health Checks (`musu list`)

Quickly see all registered applications and whether their files still exist:

```sh
musu list
```

```text
Obsidian (obsidian) target=ok desktop=ok symlink=ok
Blender (blender) target=ok desktop=ok symlink=ok
```

You can also pass `--icons` to display icon previews for listed applications in compatible terminals:

```sh
# List applications with terminal icon previews
musu list --icons
```

If you move or delete an AppImage, `musu list` flags it as `missing/broken` in red so you can easily update or fix it.

---

### Updating an Application (`musu update`)

Downloaded a newer version of an AppImage or want to change its icon? Update it without re-creating everything:

```sh
# Point to a new version
musu update obsidian --target ~/Downloads/Obsidian-1.6.0.AppImage

# Change the display name
musu update obsidian --name "Obsidian Notes"

# Change the icon
musu update obsidian --icon ~/Pictures/new-icon.png
```

---

### Removing an Application Safely (`musu remove`)

```sh
musu remove obsidian
```

**Safety guarantee:**
`musu remove` cleans up the `.desktop` launcher, copied icon, and terminal symlink. **Your original executable or AppImage is never deleted.**

---

## Isolating Applications with Sandbox

`musu` can wrap a launcher in a Bubblewrap (`bwrap`) or Firejail sandbox so the application runs with reduced privileges: optional network denial, a read-only home, and hidden folders.

The backend is auto-detected — `bwrap` first, then `firejail`; if neither is installed, apps keep running unsandboxed. Force one per app type with `backend = "bwrap" | "firejail" | "none"` inside its profile in `~/.config/musu/config.toml`.

---

### Set an app-type policy

Policies live in per-type profiles; today AppImages are the only supported type:

```toml
# ~/.config/musu/config.toml
[sandbox.appimage]
enabled = true
backend = "auto"
network = false        # every sandboxed AppImage loses network access
home_writable = false  # $HOME is mounted read-only
hide_paths = ["~/Private"]
```

Every **AppImage** registered from now on is wrapped automatically. Profiles never wrap scripts or plain binaries — pass `--sandbox` explicitly to sandbox those. See [config.md](config.md#sandbox) for all keys, defaults, and how to migrate a legacy flat `[sandbox]` config.

---

### Exempt (or restrict) a single app

Policies merge per field: what you specify for an app replaces the profile value, everything else is inherited. With the restrictive profile above, a music player that needs the network gets it back — while everything else stays offline:

```sh
musu add ~/Apps/Music.AppImage --sandbox --network true
```

The reverse works too: with the default permissive policy, tighten one app without touching the rest:

```sh
musu add ~/Apps/Scraper.AppImage --sandbox --network false --hide-path ~/Projects
```

`--hide-path`, `--extra-rw`, and `--extra-ro` (repeatable) replace the profile lists for that app. Existing apps can be re-configured the same way with `musu update`, which also regenerates their launcher.

Two rules always hold:

- `~/.ssh` and `~/.gnupg` are hidden in every sandbox — no policy or flag can expose them.
- Firejail cannot reliably enforce `home_writable = false` or `extra_rw`; `musu` warns on stderr and ignores them. Prefer `bwrap` for the full policy.

Check what an app actually gets:

```sh
musu show music
```

```text
Music (music)
  ...
Sandbox: active (bwrap) [appimage]
  ✗ ~/.ssh     → hidden
  ✗ ~/.gnupg   → hidden
```

---

### Verify it with a probe

Don't take the policy's word for it. Write a small probe script that prints whether it can reach the network, list `~/.ssh`, read a hidden folder, and write into `$HOME`, then run it inside and outside the sandbox and compare:

```sh
# 1. Write ~/scripts/probe.sh: test network, ~/.ssh, a hidden folder, $HOME writes
chmod +x ~/scripts/probe.sh

# 2. Register it with an explicit restrictive policy
musu add ~/scripts/probe.sh --name Probe --sandbox --network false --hide-path ~/Secretos

# 3. Run INSIDE the sandbox: extract the command from the generated launcher
CMD=$(grep ^Exec "$(musu show probe --desktop)" | sed "s/^Exec=//;s/^'//;s/'$//")
eval "$CMD"

# 4. Run OUTSIDE as the control group and diff the outputs
~/scripts/probe.sh
```

One detail to know before comparing: inside the sandbox `~/.ssh` (and any `hide_paths` entry) appears as an **empty directory** — a `tmpfs` is mounted over it — not as a permission error. Read-only `extra_ro` paths only fail on writes.

---

### Limitations

- **The AppImage profile only covers AppImages.** With `[sandbox.appimage] enabled = true`, newly registered AppImages are wrapped automatically; scripts and plain binaries stay unsandboxed unless you pass `--sandbox` explicitly on `musu add`/`musu update` (the explicit flag always wins, in both directions).
- **Launchers are baked at creation time.** The sandbox wrapper lives in the launcher's `Exec=` line and the generated `$PATH` wrapper script, so changing the policy (in a `config.toml` profile or per app) does **not** rewrite existing launchers automatically. After editing the `[sandbox.appimage]` profile, run `musu doctor --fix` to detect every **AppImage** whose `Exec` no longer matches the current policy and regenerate its launcher and `$PATH` wrapper in one go. Other apps (scripts, extracted binaries) are not covered by the automatic regeneration; apply the new policy manually with `musu update <app>` (e.g. `musu update music --network false`, which regenerates the launcher) or re-register with `musu add --overwrite`.
- **Only wrapped entry points run sandboxed.** Launching via the `.desktop` entry or the generated `$PATH` wrapper script runs inside the sandbox; executing the original binary directly (e.g. `~/Apps/Firefox.AppImage`) runs **without** any sandbox.
- **Firejail is best-effort.** `home_writable = false` and `extra_rw` are ignored with a warning, and `extra_ro` maps to `--blacklist`.
- **Paths with spaces are not fully escaped** in the generated launcher/wrapper line and can break it — keep registered executables on paths without spaces.

---

## Troubleshooting

### App does not appear in the application menu immediately

Most desktop environments (GNOME, KDE Plasma, XFCE) update automatically. If your menu hasn't updated yet:

- Run `update-desktop-database ~/.local/share/applications/`
- Or log out and log back in.

### Terminal says `command not found`

Ensure `~/.local/bin` is in your shell's `$PATH`. Add this to your shell config file (`~/.zshrc`, `~/.bashrc`):

```sh
export PATH="$HOME/.local/bin:$PATH"
```

### Autocompletion with Tab

`musu` automatically installs shell autocompletion. If tab completion is not active in your current session:

- In Zsh: restart your terminal or run `source ~/.zshrc`.
- Or run `musu completions <your-shell>` to print or reinstall.
