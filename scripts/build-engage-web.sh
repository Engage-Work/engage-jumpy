#!/usr/bin/env bash

#
# Builds the browser release and stages it where an Engage server serves it, so Engage's apps can
# play the game in place: <T3 home>/apps/detroit-showdown/web. With no T3 home it only builds
# web-target/wasm-release, which is that folder's contents.
#

set -euo pipefail

t3_home="${1:-}"

./scripts/build-web.sh release

if [ -n "$t3_home" ]; then
    destination="$t3_home/apps/detroit-showdown/web"
    rm -rf "$destination"
    mkdir -p "$(dirname "$destination")"
    cp -R web-target/wasm-release "$destination"
    echo "Engage can now serve the game from $destination"
fi
