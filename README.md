# QST QMC5883P Magnetometer

[![Build Status](https://github.com/sunsided/qmc5883p/workflows/Rust/badge.svg)](https://github.com/sunsided/qmc5883p/actions)
[![codecov](https://codecov.io/gh/sunsided/qmc5883p/graph/badge.svg)](https://codecov.io/gh/sunsided/qmc5883p)

> Platform-agnostic Rust support for the QST QMC5883P 3-axis magnetometer.

> **Status:** tested against a behavioral model of the chip (host tests and Wokwi), not yet
> verified on real hardware.

| Crate                                      | Description                                             |
|--------------------------------------------|---------------------------------------------------------|
| [`qmc5883p-registers`](qmc5883p-registers) | A typed map of the QMC5883P's I²C registers.            |
| [`qmc5883p-ng`](qmc5883p-ng)               | An `embedded-hal` 1.0 driver (blocking and async).      |
| [`qmc5883p-sim`](qmc5883p-sim)             | A behavioral model of the chip for tests and emulation. |

The crates.io name `qmc5883p` belongs to an unrelated crate, so the driver is published as
`qmc5883p-ng`.

The register map follows the QMC5883P datasheet (QST-PD-B002-22 Rev E); see the
[Sources](qmc5883p-registers/README.md#sources) of `qmc5883p-registers`. No code was taken
from other drivers.

## Gaps in the datasheet

The datasheet leaves some behavior open. The driver and the model assume:

- **Register `29H`** is not in the register map. The setup and self-test examples
  (section 7) write `06H` to it "to define the sign for X Y and Z axis". The driver does
  the same in `init` and `self_test`; no other value is documented.
- **Axis orientation** is unverified. Figure 2 (section 3.1, page 5) fixes +X (left) and
  +Y (towards the bottom edge) on the top face, but its diagonal Z arrow does not show
  whether +Z points up or down, and the effect of `06H` in `29H` is undefined. The
  `coordinate-frame` feature takes +Z as up (`EastNorthUp`). Check: board face-up after
  `init` in the northern hemisphere reads negative Z (about -40 µT in central Europe);
  positive Z means `NorthEastDown`.
- **Soft reset time** is not given. The driver waits the power-on reset time (250 µs).
- **Self-test limits** are not given. The driver reports the differences and leaves the
  pass/fail decision to the application. It selects the ±30 G range for both samples, which
  the datasheet's step 5 (`0BH` = `40H`) implies for the second one.
- **Reset values** of `0AH`, `0BH` and `29H`, the register pointer behavior, the
  continuous-mode rate, the single-mode conversion time and the self-test stimulus. See the
  assumptions of [`qmc5883p-sim`](qmc5883p-sim/src/lib.rs).

## Kode Dot

On the Kode Dot, the magnetometer sits at `0x2C` on I²C0 (400 kHz) without an interrupt
line, so it is polled on `DRDY`. Normal mode at 50 Hz with `OSR1` = 8
(`Config::default()`) covers 30 to 60 reads per second; `read_microtesla` returns µT.

Some boards reportedly answer at `0x0C` after some power-ups, as if bit 5 of the address
were missing. This is an unverified board report; the datasheet only gives `2CH` as the
default address and refers to the factory for others. If it happens, pass the address with
`Qmc5883p::with_address`.

## Emulation

The behavioral model in `qmc5883p-sim` is used both as a fake I²C bus in host tests and as a
[Wokwi](https://wokwi.com) custom chip (`sim/wokwi-chip`). Example firmware in
`examples/esp32c3` runs against the emulated chip using `wokwi-cli`.

**[Try it in the browser](https://wokwi.com/projects/480780869619478529)**: an ESP32-C3 talking to the emulated QMC5883P on
wokwi.com. Click the chip and use its magnetic-field sliders.

### Use the chip in your own Wokwi project

Every [GitHub Release](https://github.com/sunsided/qmc5883p/releases) carries the chip as
`chip.zip`. Add a `chip-qmc5883p` part and this dependency to your `diagram.json`, with the
latest release version written without the tag's `v` (`@v0.1.1` fails to load):

```json
"dependencies": { "chip-qmc5883p": "github:sunsided/qmc5883p@0.1.1" }
```

This works on [wokwi.com](https://wokwi.com), in the Wokwi VS Code extension and with
`wokwi-cli`; nothing needs to be built.

For a browser demo, [`examples/arduino-esp32c3`](examples/arduino-esp32c3/README.md) has an
Arduino sketch and `diagram.json` to paste into a new wokwi.com project.

### Run the example locally

Needs [`wokwi-cli`](https://docs.wokwi.com/wokwi-ci/cli-installation) and a
`WOKWI_CLI_TOKEN` from the [Wokwi CI dashboard](https://wokwi.com/dashboard/ci):

```sh
task wokwi:test
```

[`sim/wokwi-chip`](sim/wokwi-chip/README.md) describes building the chip and its pins;
[`examples/esp32c3`](examples/esp32c3/README.md) the wiring and the scenario.

## Releasing

Both crates share one version. From `main`, with the changes listed under
`## [Unreleased]` in each crate's `CHANGELOG.md`:

```sh
cargo release <patch|minor|major>            # dry run
cargo release <patch|minor|major> --execute
```

This commits `Release X.Y.Z` and pushes the tag `vX.Y.Z`. The tag runs the release workflow,
which publishes `qmc5883p-registers` and `qmc5883p-ng` to crates.io via
[trusted publishing](https://crates.io/docs/trusted-publishing) (no API token in the
repository) and then creates a GitHub Release named `X.Y.Z` with the changelog sections
as notes.

Each GitHub Release carries the packaged `.crate` files and the Wokwi chip as `chip.zip`,
all with [build provenance attestations](https://docs.github.com/actions/security-for-github-actions/using-artifact-attestations):

```sh
gh attestation verify chip.zip -R sunsided/qmc5883p
```

## Code of Conduct

We abide by the [Contributor Covenant][cc] and ask that you do as well.

## License

Copyright © 2026 Markus Mayer

Triple licensed under your choice of either of:

- European Union Public Licence, Version 1.2, ([LICENSE-EUPL](LICENSE-EUPL)
  or https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12)
- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

[cc]: https://contributor-covenant.org
