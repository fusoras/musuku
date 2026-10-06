# User Guide

`geico` integrates portable applications—AppImages, standalone binaries, extracted archives, and custom scripts—into your desktop application menu and terminal `$PATH` with a single command.

---

## Why Use Geico?

When you download a portable Linux application (like an AppImage or a tarball), Linux does not automatically:
1. Add an icon to your application launcher or dock.
2. Allow you to run it from any terminal by typing its name.

`geico` solves this safely in user-space (`~/.local`), without requiring `sudo` privileges or touching your original application files.

---

## Common Scenarios

### 1. Integrating an AppImage

Downloaded an AppImage like Obsidian, Krita, or Cursor?

```sh
# Basic integration
geico add ~/Downloads/Obsidian-1.5.8.AppImage

# With a custom icon and desktop categories
geico add ~/Downloads/Obsidian-1.5.8.AppImage \
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
geico add /opt/blender-4.2/blender \
  --name "Blender" \
  --icon /opt/blender-4.2/blender.svg \
  --categories "Graphics,3DGraphics"
```

---

### 3. Integrating a Custom Shell Script

Have a personal script you want available globally in your terminal and launcher?

```sh
geico add ~/scripts/backup-notes.sh \
  --name "Backup Notes" \
  --bin-name "backup-notes" \
  --categories "Utility"
```

---

## Managing Your Applications

### Check App Details and Paths (`geico show`)

Inspect the exact file locations and health of any registered application:

```sh
geico show obsidian
```

Output:
```text
Obsidian (obsidian)
  Executable:  /home/user/Downloads/Obsidian-1.5.8.AppImage [ok]
  Desktop:     /home/user/.local/share/applications/obsidian.desktop [ok]
  Icon:        /home/user/.local/share/icons/obsidian.png [ok]
  Symlink:     /home/user/.local/bin/obsidian [ok]
  Categories:  Office, Utility
  Registered:  2026-10-06 14:00:00 UTC
```

You can also extract a single path for scripts or terminal shortcuts:
```sh
# Print only the executable path
geico show obsidian --bin

# Print only the .desktop launcher path
geico show obsidian --desktop

# Output as JSON
geico show obsidian --json
```

In terminals that support image previews (such as Kitty, Ghostty, WezTerm, and Herdr),
`geico show` displays the application icon inline above the card by default. Pass
`--no-image` to keep the output as plain text, or `-i` to force it:

```sh
# Force the text card, skipping any terminal image preview
geico show obsidian --no-image

# Explicitly request icon preview
geico show obsidian -i
```

---

### List All Apps & Health Checks (`geico list`)

Quickly see all registered applications and whether their files still exist:

```sh
geico list
```

```text
Obsidian (obsidian) target=ok desktop=ok symlink=ok
Blender (blender) target=ok desktop=ok symlink=ok
```

You can also pass `-i` or `--image` to display icon previews for listed applications in compatible terminals:

```sh
# List applications with terminal icon previews
geico list -i
```

If you move or delete an AppImage, `geico list` flags it as `missing/broken` in red so you can easily update or fix it.

---

### Updating an Application (`geico update`)

Downloaded a newer version of an AppImage or want to change its icon? Update it without re-creating everything:

```sh
# Point to a new version
geico update obsidian --target ~/Downloads/Obsidian-1.6.0.AppImage

# Change the display name
geico update obsidian --name "Obsidian Notes"

# Change the icon
geico update obsidian --icon ~/Pictures/new-icon.png
```

---

### Removing an Application Safely (`geico remove`)

```sh
geico remove obsidian
```

**Safety guarantee:**
`geico remove` cleans up the `.desktop` launcher, copied icon, and terminal symlink. **Your original executable or AppImage is never deleted.**

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
`geico` automatically installs shell autocompletion. If tab completion is not active in your current session:
- In Zsh: restart your terminal or run `source ~/.zshrc`.
- Or run `geico completions <your-shell>` to print or reinstall.
