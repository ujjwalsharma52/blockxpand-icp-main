#!/usr/bin/env bash
set -euo pipefail

# Install dfx locally if it is not already available.
# Uses DFINITY's install script in non-interactive mode.

DFX_VERSION="${DFX_VERSION:-0.17.0}"

if command -v dfx >/dev/null 2>&1; then
    echo "dfx already installed: $(dfx --version)"
    exit 0
fi

install_from_tarball() {
    local tarball="$1"
    local dir
    dir=$(mktemp -d)
    tar -xzf "$tarball" -C "$dir"
    mkdir -p "$HOME/.local/bin"
    cp "$dir"/dfx "$HOME/.local/bin/dfx"
    chmod +x "$HOME/.local/bin/dfx"
    rm -rf "$dir"
}

if [ -n "${DFX_TARBALL:-}" ] && [ -f "$DFX_TARBALL" ]; then
    echo "Installing dfx from $DFX_TARBALL..."
    install_from_tarball "$DFX_TARBALL"
elif [ -n "${DFX_INSTALL_INSECURE:-}" ]; then
    echo "Installing dfx ${DFX_VERSION} (insecure download)..."
    arch="x86_64-linux"
    tmp=$(mktemp -d)
    archive="dfx-${DFX_VERSION}-${arch}.tar.gz"
    url="https://github.com/dfinity/sdk/releases/download/${DFX_VERSION}/${archive}"
    curl -sL -k "$url" -o "$tmp/$archive"
    curl -sL -k "${url}.sha256" -o "$tmp/$archive.sha256"
    (cd "$tmp" && sha256sum -c "$archive.sha256")
    install_from_tarball "$tmp/$archive"
else
    echo "Installing dfx ${DFX_VERSION}..."
    DFXVM_INIT_YES=1 DFX_VERSION="$DFX_VERSION" sh -ci "$(curl -fsSL https://internetcomputer.org/install.sh)" >/dev/null
fi

echo "dfx $(dfx --version) installed"
