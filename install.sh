#!/bin/sh
# Installs seatdeck-cli from the latest GitHub release into ~/.local/bin.
#   curl -fsSL https://raw.githubusercontent.com/little-u-private-project/claude-account-switcher/main/install.sh | sh
set -eu

repo="${SEATDECK_REPO:-little-u-private-project/claude-account-switcher}"
dir="${SEATDECK_BIN_DIR:-$HOME/.local/bin}"

case "$(uname -s)-$(uname -m)" in
  Darwin-*) asset=seatdeck-cli-macos-universal ;;
  Linux-x86_64 | Linux-amd64) asset=seatdeck-cli-linux-x64 ;;
  *) echo "No prebuilt seatdeck-cli for $(uname -s) $(uname -m) - build it from cli/ with cargo." >&2; exit 1 ;;
esac

mkdir -p "$dir"
curl -fSL --progress-bar "https://github.com/$repo/releases/latest/download/$asset" -o "$dir/seatdeck-cli.part"
chmod +x "$dir/seatdeck-cli.part"
mv "$dir/seatdeck-cli.part" "$dir/seatdeck-cli"
echo "Installed $("$dir/seatdeck-cli" --version) to $dir"

case ":$PATH:" in
  *":$dir:"*) ;;
  *) echo "Add $dir to your PATH, e.g.: echo 'export PATH=\"$dir:\$PATH\"' >> ~/.profile" ;;
esac
