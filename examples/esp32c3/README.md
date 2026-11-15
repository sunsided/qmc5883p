# QMC5883P on ESP32-C3

Reads a QMC5883P with the blocking `qmc5883p-ng` driver: checks the chip ID, runs the
self-test, configures normal mode at 50 Hz with `OSR1` = 8 and ±8 G, then polls `DRDY`
and prints field (µT) and heading whenever they change. The setup mirrors the Kode Dot,
where the chip has no interrupt line.

| ESP32-C3 | QMC5883P |
|----------|----------|
| GPIO4    | SDA      |
| GPIO5    | SCL      |

## Run in Wokwi

Requires `rustup target add riscv32imc-unknown-none-elf wasm32-unknown-unknown`,
[`wokwi-cli`](https://docs.wokwi.com/wokwi-ci/cli-installation) and `WOKWI_CLI_TOKEN`.

```sh
../../sim/wokwi-chip/build.sh
cargo build --release
wokwi-cli . --scenario scenarios/compass.yaml --timeout 30000
```

Or open this directory in VS Code with the Wokwi extension and drag the field sliders.

## Run on hardware

```sh
cargo run --release   # uses espflash
```
