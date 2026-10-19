# crunchatize

Cap'n Crunch's Crunchling Adventure (1999) rebuilt in Rust from the original binaries and compiled to WebAssembly, playing from the game's own data files, with a Go Online mode that lets a friend join your home world: https://crunchatize.app

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

# desktop build, --online picks the game server for the Go Online button
cargo run --release -p crunch-native -- --data /path/to/CrunchData --data assets --scale 2 [--online wss://crunchatize.app]

# deterministic headless run with frame dumps and mixed audio
cargo run --release -p crunch-native -- --data /path/to/CrunchData --data assets --headless --ticks 300 --out frames --script input.txt --wav out.wav

# rebuild the Go Online art, font, cursor and scene in assets/ from the original data
tools/make-online-assets.py /path/to/CrunchData assets
```
