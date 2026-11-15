#!/usr/bin/env sh
# Builds the chip and places it next to qmc5883p.chip.json, as Wokwi expects.
set -eu
cd "$(dirname "$0")"
cargo build --release
cp target/wasm32-unknown-unknown/release/qmc5883p_wokwi_chip.wasm qmc5883p.chip.wasm
