#!/bin/sh
set -e
cd "$(dirname "$0")"
cargo build --release
../node_modules/.bin/wasm-opt -O3 --enable-simd --enable-bulk-memory --enable-sign-ext --enable-nontrapping-float-to-int --enable-mutable-globals \
  --strip-debug --strip-producers target/wasm32-unknown-unknown/release/wasm_core.wasm -o js/quicktex.wasm
ls -l js/quicktex.wasm

cargo build --manifest-path ../wasm-fonts/Cargo.toml --target wasm32-unknown-unknown --release
../node_modules/.bin/wasm-opt -O3 --enable-simd --enable-bulk-memory --enable-sign-ext --enable-nontrapping-float-to-int --enable-mutable-globals --strip-debug --strip-producers ../wasm-fonts/target/wasm32-unknown-unknown/release/quicktex_fonts.wasm -o js/quicktex-fonts.wasm
