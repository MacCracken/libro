#!/usr/bin/env bash
set -euo pipefail
[ $# -ne 1 ] && echo "Usage: $0 <version>" && exit 1
NEW_VERSION="$1"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
echo "$NEW_VERSION" > "$REPO_ROOT/VERSION"
MANIFEST="$REPO_ROOT/cyrius.cyml"
[ -f "$MANIFEST" ] || MANIFEST="$REPO_ROOT/cyrius.toml"
sed -i "s/^version = \".*\"/version = \"${NEW_VERSION}\"/" "$MANIFEST"
echo "Bumped to ${NEW_VERSION} (manifest: $MANIFEST). Tag and push."
