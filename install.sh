#!/bin/sh
set -e

# musu - installer script
# Installs musu into user-space (~/.local/bin) and configures completions.

SCRIPT_DIR="$(cd "$(dirname "$0")" 2>/dev/null && pwd || echo "")"
BIN_DIR="${MUSU_INSTALL_DIR:-$HOME/.local/bin}"
TARGET_BIN="$BIN_DIR/musu"

mkdir -p "$BIN_DIR"

# 1. Locate or obtain the musu binary
if [ -f "$SCRIPT_DIR/target/release/musu" ]; then
    cp "$SCRIPT_DIR/target/release/musu" "$TARGET_BIN"
elif [ -f "$SCRIPT_DIR/target/debug/musu" ]; then
    cp "$SCRIPT_DIR/target/debug/musu" "$TARGET_BIN"
elif [ -f "$SCRIPT_DIR/musu" ]; then
    cp "$SCRIPT_DIR/musu" "$TARGET_BIN"
elif [ -f "$SCRIPT_DIR/Cargo.toml" ] && command -v cargo >/dev/null 2>&1; then
    cargo build --release --manifest-path "$SCRIPT_DIR/Cargo.toml" --quiet
    cp "$SCRIPT_DIR/target/release/musu" "$TARGET_BIN"
elif [ -f "$(command -v musu 2>/dev/null)" ] && [ "$(command -v musu)" != "$TARGET_BIN" ]; then
    cp "$(command -v musu)" "$TARGET_BIN"
else
    OS="$(uname -s)"
    ARCH="$(uname -m)"

    if [ "$OS" != "Linux" ]; then
        echo "Error: musu is only supported on Linux (detected: $OS)" >&2
        exit 1
    fi

    # Map architecture
    case "$ARCH" in
        x86_64) ARCH_SUFFIX="x86_64-unknown-linux-gnu" ;;
        aarch64|arm64) ARCH_SUFFIX="aarch64-unknown-linux-gnu" ;;
        *)
            echo "Error: unsupported architecture: $ARCH" >&2
            exit 1
            ;;
    esac

    REPO="fusoras/musu"
    DOWNLOAD_URL="https://github.com/$REPO/releases/latest/download/musu-$ARCH_SUFFIX.tar.gz"

    TMP_DIR="$(mktemp -d 2>/dev/null || mktemp -d -t 'musu')"
    TARBALL="$TMP_DIR/musu.tar.gz"

    if command -v curl >/dev/null 2>&1; then
        curl -# -fLo "$TARBALL" "$DOWNLOAD_URL"
    elif command -v wget >/dev/null 2>&1; then
        wget -q --show-progress -O "$TARBALL" "$DOWNLOAD_URL"
    else
        echo "Error: curl or wget is required to download musu." >&2
        exit 1
    fi

    tar -xzf "$TARBALL" -C "$TMP_DIR"
    cp "$TMP_DIR/musu" "$TARGET_BIN"
    rm -rf "$TMP_DIR"
fi

chmod +x "$TARGET_BIN"

# 2. Trigger auto-completion registration
"$TARGET_BIN" completions >/dev/null 2>&1 || true

# 3. Detect Shell and configuration RC file
SHELL_NAME="$(basename "${SHELL:-bash}")"
case "$SHELL_NAME" in
    zsh)
        RC_FILE="~/.zshrc"
        RC_LABEL="~/.zshrc (or similar)"
        ;;
    fish)
        RC_FILE="~/.config/fish/config.fish"
        RC_LABEL="~/.config/fish/config.fish (or similar)"
        ;;
    bash)
        RC_FILE="~/.bashrc"
        RC_LABEL="~/.bashrc (or similar)"
        ;;
    *)
        RC_FILE="~/.profile"
        RC_LABEL="~/.zshrc (or similar)"
        ;;
esac

# 4. Check if BIN_DIR is present in $PATH
IN_PATH=0
case ":$PATH:" in
    *":$BIN_DIR:"*) IN_PATH=1 ;;
esac

# Check canonical / home-relative representation
if [ "$IN_PATH" -eq 0 ] && [ -n "$HOME" ]; then
    REL_BIN="${BIN_DIR#$HOME/}"
    if [ "$REL_BIN" != "$BIN_DIR" ]; then
        # Try checking with literal $HOME or ~
        case ":$PATH:" in
            *":$HOME/$REL_BIN:"*|*":~/$REL_BIN:"*) IN_PATH=1 ;;
        esac
    fi
fi

# Formatted display strings
if [ -n "$HOME" ] && [ "${TARGET_BIN#$HOME/}" != "$TARGET_BIN" ]; then
    DISPLAY_TARGET="~/${TARGET_BIN#$HOME/}"
    DISPLAY_DIR="\$HOME/${BIN_DIR#$HOME/}"
else
    DISPLAY_TARGET="$TARGET_BIN"
    DISPLAY_DIR="$BIN_DIR"
fi

# 5. Silently create background service unit if systemd is available
if command -v systemctl >/dev/null 2>&1 && [ -n "$HOME" ]; then
    SYSTEMD_USER_DIR="$HOME/.config/systemd/user"
    mkdir -p "$SYSTEMD_USER_DIR"
    cat << EOF > "$SYSTEMD_USER_DIR/musu-watch.service"
[Unit]
Description=Musu AppImage Folder Watcher & Desktop Integrator
Documentation=https://github.com/fusoras/musu
After=default.target

[Service]
Type=simple
ExecStart=$TARGET_BIN watch --continuous
Restart=on-failure
RestartSec=5s

[Install]
WantedBy=default.target
EOF
fi

# 6. Output Bun-style installation notice
echo "######################################################################## 100.0%"
echo "musu was installed successfully to $DISPLAY_TARGET"
echo ""

if [ "$IN_PATH" -eq 0 ]; then
    echo "Manually add the directory to $RC_LABEL:"
    if [ "$SHELL_NAME" = "fish" ]; then
        echo "  fish_add_path \"$DISPLAY_DIR\""
    else
        echo "  export PATH=\"$DISPLAY_DIR:\$PATH\""
    fi
    echo ""
fi

if [ -f "$HOME/.config/systemd/user/musu-watch.service" ]; then
    echo "To enable background folder watching (optional):"
    echo "  systemctl --user daemon-reload"
    echo "  systemctl --user enable --now musu-watch.service"
    echo ""
fi

echo "To get started, run:"
echo ""
echo "  musu --help"
