# User Guide

`musu` integrates portable applications—AppImages, standalone binaries, extracted archives, and custom scripts—into your desktop application menu and terminal `$PATH` with a single command.

---

## Why Use Musu?

When you download a portable Linux application (like an AppImage or a tarball), Linux does not automatically:
1. Add an icon to your application launcher or dock.
2. Allow you to run it from any terminal by typing its name.

`musu` solves this safely in user-space (`~/.local`), without requiring `sudo` privileges or touching your original application files.

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
`--no-image` to keep the output as plain text, or `-i` to force it:

```sh
# Force the text card, skipping any terminal image preview
musu show obsidian --no-image

# Explicitly request icon preview
musu show obsidian -i
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

You can also pass `-i` or `--image` to display icon previews for listed applications in compatible terminals:

```sh
# List applications with terminal icon previews
musu list -i
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
