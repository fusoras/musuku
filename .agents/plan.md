# Plan: `geico watch` & Icon Folder Integration

**Status:** completed.
**Date:** 2026-10-07
**Objective:** Add folder monitoring and batch scanning for AppImages with automated icon pairing and configuration support.

---

## Agreed Requirements & Decisions

1. **CLI Command**:
   - `geico watch [DIRECTORIES]... [OPTIONS]`
   - Flag `--scan-only` (alias `--once`): Scans the directories, registers any new AppImages, and exits immediately.
   - Default without `--scan-only`: Performs the initial scan and then continuously monitors filesystem events with debouncing.
   - Additional flags: `--icons-dir <DIR>`, `--recursive`, `--dry-run`.
2. **Icon Pairing & Fallback**:
   - When an AppImage is detected, search the dedicated icons directory for a matching icon by app/stem name (`.png`, `.svg`, `.webp`, `.xpm`, `.ico`).
   - If not found or no icons folder specified, **fallback to integrated extraction** (`extractor::inspect_and_extract` / `.DirIcon` / internal `.desktop`).
3. **Configuration (`config.toml`)**:
   - Add `[watch]` section to `~/.config/geico/config.toml` to define default watch directories, icons directory, auto-chmod flag, clean-on-delete, and debounce duration.
4. **Lifecycle & Deletion Behavior (Option A)**:
   - Non-destructive retention: AppImages remain in the watched directory.
   - When an AppImage is deleted or moved out of the watched directory by the user, `geico watch` detects the event and automatically unregisters it (removes `.desktop`, symlink, and entry in `apps.toml`).
5. **Execution Permissions**:
   - Do not force `chmod +x` by default.
   - Configurable via `auto_chmod = true` in `[watch]` section of `config.toml`.

---

## Configuration Specification

In `~/.config/geico/config.toml`:

```toml
[watch]
# Default directories to watch if none provided to `geico watch`
directories = ["~/Applications"]

# Dedicated icons folder to match against AppImage names
icons_dir = "~/Applications/Icons"

# Automatically apply `chmod +x` to detected AppImages (default: false)
auto_chmod = false

# Monitor subdirectories recursively (default: false)
recursive = false

# Unregister application (.desktop, symlink, apps.toml) when AppImage is deleted (default: true)
clean_on_delete = true

# Event debounce delay in milliseconds (default: 1000)
debounce_ms = 1000
```

---

## Architecture & Code Changes

### 1. Dependencies (`Cargo.toml`)
- Add `notify-debouncer-mini = "0.7"` (or `notify = "8.0"`) for debounced filesystem event monitoring in Linux.

### 2. CLI Definition (`src/cli.rs`)
- Add `Command::Watch(Watch)`:
  - `directories`: `Vec<PathBuf>` (positional, optional, falls back to config).
  - `icons_dir`: `Option<PathBuf>`.
  - `scan_only`: `bool` (`--scan-only`, alias `--once`).
  - `recursive`: `bool` (`--recursive`).
  - `dry_run`: `bool` (`--dry-run`).

### 3. Config Model (`src/config.rs`)
- Add `WatchConfig` struct with serde defaults inside `Settings`.

### 4. Icon & Name Matcher (`src/matcher.rs` - NEW)
- Name normalization: Clean version suffixes (`-1.0.0`, `_v2`), architectures (`-x86_64`, `_amd64`), and extract search slugs.
- Icon finder: Scan `--icons-dir` for matching stems (`<name>.{png,svg,webp,xpm,ico}`). Case-insensitive and normalized.
- Fallback orchestration: If icon matched $\rightarrow$ use it. If not $\rightarrow$ `None` (triggers `extractor::inspect_and_extract`).

### 5. Watcher & Scanner Engine (`src/watcher.rs` - NEW)
- `scan_directory`: Recursively/flatly collect AppImages, skip already registered paths, resolve icons, and call `manager.add()`.
- `watch_directory`: Initialize debounced inotify watcher. On `Create`/`Modify` $\rightarrow$ register. On `Remove` $\rightarrow$ if `clean_on_delete`, unregister via `manager.remove()`.
- Unix permission check: Apply `chmod +x` only if `auto_chmod == true`.

### 6. CLI Dispatcher (`src/main.rs`)
- Wire `Command::Watch` to `watcher::run(args, manager)`.

### 7. Documentation (`docs/watch.md` & `README.md`)
- Document the new `watch` command, flags, and `[watch]` configuration options.

---

## Implementation Checklist

- [x] Add `notify-debouncer-mini` (or `notify`) to `Cargo.toml`.
- [x] Implement `WatchConfig` in `src/config.rs`.
- [x] Add `Watch` command and arguments in `src/cli.rs`.
- [x] Implement `src/matcher.rs` with unit tests for name cleanup and icon lookup.
- [x] Implement `src/watcher.rs` with batch scanning and debounced live watching.
- [x] Integrate optional `auto_chmod` and reactive `clean_on_delete`.
- [x] Wire `Command::Watch` in `src/main.rs`.
- [x] Write automated tests in `tests/watch_and_matcher_test.rs`.
- [x] Verify test suite with `cargo test` and `cargo clippy`.
- [x] Update documentation (`docs/watch.md` and `README.md`).
