#!/bin/sh
# Task 3.6: run packaging/install-extension.sh against a temporary
# XDG_DATA_HOME (and in-memory GSettings), then check what it installed.
# Run from the repo root in the `dev` toolbox:
#
#   toolbox run -c dev sh openspec/changes/add-quick-settings-tile/evidence/install_check.sh
set -eu
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
# gnome-extensions unpacks into the cache dir and moves the result, so both
# must be on one filesystem, as ~/.cache and ~/.local/share are.
export XDG_DATA_HOME="$tmp/data" XDG_CACHE_HOME="$tmp/cache" GSETTINGS_BACKEND=memory
sh packaging/install-extension.sh
dir="$tmp/data/gnome-shell/extensions/g512-control@g512control.github.io"
echo "--- installed files:"
(cd "$dir" && find . -type f | sort)
echo "--- metadata:"
python3 -I -c 'import json, sys; m = json.load(open(sys.argv[1])); print(m["uuid"], m["shell-version"])' "$dir/metadata.json"
