#!/usr/bin/env bash
set -euo pipefail
[ $# -ne 1 ] && echo "Usage: $0 <version>" && exit 1
NEW_VERSION="$1"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
echo "$NEW_VERSION" > "$REPO_ROOT/VERSION"
MANIFEST="$REPO_ROOT/cyrius.cyml"
[ -f "$MANIFEST" ] || MANIFEST="$REPO_ROOT/cyrius.toml"
# VERSION is the SINGLE SOURCE OF TRUTH. If the manifest references it via
# ${file:VERSION} (the convention), leave that line ALONE — cyrius resolves it
# from VERSION at build time. Only rewrite a hardcoded literal (legacy manifest);
# clobbering ${file:VERSION} with a literal is what breaks single-source-of-truth.
if grep -qE '^version = "\$\{file:VERSION\}"' "$MANIFEST"; then
    echo "Manifest version uses \${file:VERSION} — left untouched (VERSION is the source of truth)."
else
    sed -i "s/^version = \".*\"/version = \"${NEW_VERSION}\"/" "$MANIFEST"
fi
echo "Bumped VERSION to ${NEW_VERSION} (manifest: $MANIFEST). Tag and push."
