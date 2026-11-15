# QST QMC5883P Magnetometer Registers

[![Crates.io][crates-image]][crates-link]
[![Docs][docs-image]][docs-link]
[![Build Status][build-image]][build-link]
[![Coverage][codecov-image]][codecov-link]
[![Safety Dance][safety-image]][safety-link]
![MSRV][msrv-image]
[![EUPL 1.2 licensed][license-eupl-image]][license-eupl-link]
[![Apache 2.0 licensed][license-apache-image]][license-apache-link]
[![MIT licensed][license-mit-image]][license-mit-link]

> A typed map of the QMC5883P's I²C-accessible registers.

Provides functions and named fields to read and modify the magnetometer registers,
built on [`hardware-registers`](https://crates.io/crates/hardware-registers).
The register map follows the QMC5883P datasheet (QST-PD-B002-22 Rev E).

## Sources

All facts come from the QST QMC5883P datasheet, document QST-PD-B002-22 Rev E (April 2025).

| Fact | Source |
|------|--------|
| Default 7-bit I²C address `2CH`; 100 kHz and 400 kHz | QST-PD-B002-22 Rev E, section 5.4 |
| Chip ID register `00H`, value `80H` | QST-PD-B002-22 Rev E, sections 9.1 and 9.2.1 |
| Output `01H`–`06H`: X, Y, Z, LSB first, 16-bit two's complement, saturating at -32768 and 32767 | QST-PD-B002-22 Rev E, section 9.2.1, table 15 |
| Status `09H`: `DRDY` bit 0, `OVFL` bit 1, both reset by reading | QST-PD-B002-22 Rev E, section 9.2.2, table 16 |
| `OVFL` when an axis exceeds [-30000, 30000] LSB | QST-PD-B002-22 Rev E, section 9.2.2 |
| Control 1 `0AH`: `MODE` [1:0], `ODR` [3:2], `OSR1` [5:4], `OSR2` [7:6] and their values | QST-PD-B002-22 Rev E, section 9.2.3, table 17 |
| Control 2 `0BH`: set/reset mode [1:0], `RNG` [3:2], `SELF_TEST` bit 6, `SOFT_RST` bit 7 | QST-PD-B002-22 Rev E, section 9.2.3, table 18 |
| Modes: suspend (default after POR and soft reset), normal, single, continuous; suspend in between mode changes | QST-PD-B002-22 Rev E, sections 6 and 9.2.3 |
| Sensitivity 1000, 2500, 3750, 15000 LSB/G for ±30, ±12, ±8, ±2 G | QST-PD-B002-22 Rev E, section 2.1, table 2 |
| Axis sign register `29H`, written with `06H` | QST-PD-B002-22 Rev E, sections 7.1–7.3 (not in the register map) |
| Self-test procedure: continuous mode, `0BH` = `40H`, wait 5 ms, differences `x1-x2`, `y2-y1`, `z2-z1` | QST-PD-B002-22 Rev E, section 7.3 |
| Self-test only in continuous mode, clears itself | QST-PD-B002-22 Rev E, sections 6.2.3 and 9.2.3, table 18 |
| Power-on reset time 250 µs | QST-PD-B002-22 Rev E, section 5.3, table 7 |

The datasheet documents no bit layout or reset value for `29H`, and no reset values for
`0AH` and `0BH` beyond suspend mode. This crate assumes `00H` for both control registers.

See also the [`qmc5883p-ng`](https://crates.io/crates/qmc5883p-ng) driver.

## License

Copyright © 2026 Markus Mayer

Triple licensed under your choice of either of:

- European Union Public Licence, Version 1.2, ([LICENSE-EUPL](LICENSE-EUPL)
  or https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12)
- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

[crates-image]: https://img.shields.io/crates/v/qmc5883p-registers
[crates-link]: https://crates.io/crates/qmc5883p-registers
[docs-image]: https://docs.rs/qmc5883p-registers/badge.svg
[docs-link]: https://docs.rs/qmc5883p-registers/
[build-image]: https://github.com/sunsided/qmc5883p/workflows/Rust/badge.svg
[build-link]: https://github.com/sunsided/qmc5883p/actions
[codecov-image]: https://codecov.io/gh/sunsided/qmc5883p/graph/badge.svg
[codecov-link]: https://codecov.io/gh/sunsided/qmc5883p
[safety-image]: https://img.shields.io/badge/unsafe-forbidden-success.svg
[safety-link]: https://github.com/rust-secure-code/safety-dance/
[msrv-image]: https://img.shields.io/badge/rustc-1.85+-blue.svg
[license-eupl-image]: https://img.shields.io/badge/license-EUPL_1.2-blue.svg
[license-eupl-link]: https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12
[license-apache-image]: https://img.shields.io/badge/license-Apache_2.0-blue.svg
[license-apache-link]: https://www.apache.org/licenses/LICENSE-2.0
[license-mit-image]: https://img.shields.io/badge/license-MIT-blue.svg
[license-mit-link]: https://opensource.org/licenses/MIT
