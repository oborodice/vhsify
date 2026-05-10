#!/bin/bash
set -e

VERSION=$1

if [ -z "$VERSION" ]; then
    echo "Usage: bash scripts/release.sh <version>"
    echo "Example: bash scripts/release.sh 0.2.0"
    exit 1
fi

perl -i -pe "s/^version = .*/version = \"$VERSION\"/" Cargo.toml
git add Cargo.toml Cargo.lock
git commit -m "chore: bump version to v$VERSION"
git tag "v$VERSION"
git push
git push origin "v$VERSION"
