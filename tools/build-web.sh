#!/usr/bin/env sh
# Builds the wasm package into web/pkg so web/ can be served as a static site
set -eu
cd "$(dirname "$0")/.."
wasm-pack build crates/crunch-web --release --target web --out-dir ../../web/pkg --no-typescript
rm -f web/pkg/.gitignore web/pkg/package.json
ls web/pkg
