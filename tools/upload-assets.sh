#!/usr/bin/env sh
# Packs the game data folder and uploads the pack to the bucket the worker reads, --local targets wrangler dev
set -eu
src="${1:?usage: upload-assets.sh <CrunchData dir> [--local]}"
target="${2:---remote}"
cd "$(dirname "$0")/.."
cargo run --release -p crunch-pak -- "$src" target/crunch.pak
npx wrangler r2 object put crunchatize-data/crunch.pak --file target/crunch.pak "$target"
