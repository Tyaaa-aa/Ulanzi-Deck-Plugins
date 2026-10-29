#!/usr/bin/env bash
# Release every plugin whose plugin.toml version has no GitHub release yet (tag <id>-v<version>):
# build, package, sign, publish. Then rebuild index.json from the current version of every plugin,
# sign it and put both on the "index" release, where the app reads them.
#
# Run by CI on main with GH_TOKEN, MINISIGN_KEY and MINISIGN_PASSWORD set (see README).
set -euo pipefail
cd "$(dirname "$0")/.."
repo="${GITHUB_REPOSITORY:?run in CI, or set GITHUB_REPOSITORY=owner/name}"
target="${GITHUB_SHA:-$(git rev-parse HEAD)}"

sign() { cargo run -q --release --locked -p ulanzi-release -- sign "$1"; }
# A top-level string key of a plugin.toml (before the first [table]).
value() { sed -n '/^\[/q;s/^'"$2"'[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$1/plugin.toml" | head -n 1; }

index_dir=dist/index
mkdir -p "$index_dir"
for dir in plugins/*/; do
  dir=${dir%/}
  id=$(value "$dir" id)
  version=$(value "$dir" version)
  name=$(value "$dir" name)
  tag="$id-v$version"
  if gh release view "$tag" -R "$repo" >/dev/null 2>&1; then
    echo "$tag: already released"
  else
    echo "$tag: releasing"
    pkg=$(scripts/package-plugin.sh "$dir")
    sign "$pkg"
    gh release create "$tag" "$pkg" "$pkg.minisig" -R "$repo" --target "$target" \
      --title "$name $version" --latest=false \
      --notes "$name $version for Ulanzi Deck. Install it from the app's Plugins window, or download the .ulanziplugin file and choose Install from File. Signed: the app checks the .minisig before installing."
  fi
  gh release download "$tag" -R "$repo" -D "$index_dir" -p '*.ulanziplugin' --clobber
done

cargo run -q --release --locked -p ulanzi-release -- index "$index_dir" --repo "$repo" > dist/index.json
sign dist/index.json
if ! gh release view index -R "$repo" >/dev/null 2>&1; then
  gh release create index -R "$repo" --target "$target" --title "Plugin index" --latest=false \
    --notes "index.json lists the current version of every plugin; the app reads it and checks index.json.minisig. Updated automatically."
fi
gh release upload index dist/index.json dist/index.json.minisig -R "$repo" --clobber
echo "index updated"
