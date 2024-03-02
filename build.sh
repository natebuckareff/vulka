#!/usr/bin/env bash
set -euo pipefail
TARGET="$1"
VERSION=$(git describe --tags --exact-match)
case "$TARGET" in
    win-x86_64);;
    linux-x86_64);;
    *)
        echo "unknown target" >&2
        exit1
        ;;
esac
docker build --build-arg="VERSION=${VERSION}" -f "./dockerfile.$TARGET" --progress=plain .
id=$(docker create $(docker build --build-arg="VERSION=${VERSION}" -f "./dockerfile.$TARGET" -q .))
docker cp "$id:/rust/build/vulka-${VERSION}-${TARGET}.zip" .