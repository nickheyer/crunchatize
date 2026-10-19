# crunchatize

Cap'n Crunch's Crunchling Adventure (1999) rebuilt in Rust from the original binaries and compiled to WebAssembly, playing from the game's own data files: https://crunchatize.app

## Install

```sh
cargo install wasm-pack
npm install
npx wrangler r2 bucket create crunchatize-data
tools/upload-assets.sh /path/to/CrunchData
tools/build-web.sh
```

## Usage

```sh
# local site at http://localhost:8787
tools/upload-assets.sh /path/to/CrunchData --local
npx wrangler dev

# deploy to crunchatize.app, also run by the GitHub action on every push to main
npx wrangler deploy

# desktop build
cargo run --release -p crunch-native -- --data /path/to/CrunchData --scale 2

# deterministic headless run with frame dumps and mixed audio
cargo run --release -p crunch-native -- --data /path/to/CrunchData --headless --ticks 300 --out frames --script input.txt --wav out.wav
```
