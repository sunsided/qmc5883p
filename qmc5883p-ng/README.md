# QST QMC5883P Magnetometer Driver

[![Crates.io][crates-image]][crates-link]
[![Docs][docs-image]][docs-link]
[![Build Status][build-image]][build-link]
[![Coverage][codecov-image]][codecov-link]
[![Safety Dance][safety-image]][safety-link]
![MSRV][msrv-image]
[![EUPL 1.2 licensed][license-eupl-image]][license-eupl-link]
[![Apache 2.0 licensed][license-apache-image]][license-apache-link]
[![MIT licensed][license-mit-image]][license-mit-link]

> A platform-agnostic `embedded-hal` 1.0 driver for the QMC5883P 3-axis magnetometer.

> **Status:** tested against a behavioral model of the chip (host tests and Wokwi), not yet
> verified on real hardware.

- `no_std`, no `unsafe`, I²C. The bus is taken by value, so `embedded-hal-bus` sharing works.
- Blocking API, plus an equivalent async API on `embedded-hal-async` behind the `async` feature
- Chip ID check, soft reset, and the axis sign setup of the datasheet
- Suspend, normal, single and continuous mode; output data rate, `OSR1`, `OSR2`, range
  and set/reset mode
- Data-ready and overflow polling
- Magnetic field in raw LSB, gauss or microtesla, converted with the configured range
- Self-test following the datasheet procedure (section 7.3)
- Typed access to every register via `read_register`, `write_register` and `modify_register`
- Waits use `embedded_hal::delay::DelayNs` (or its async twin); no timer crate is required

```rust,ignore
use qmc5883p_ng::{Config, Qmc5883p};

let mut mag = Qmc5883p::new(i2c); // or Qmc5883p::with_address(i2c, address)
assert!(mag.identify()?);
mag.init(&mut delay)?;
mag.configure(Config::default())?; // normal mode, 50 Hz, OSR1 = 8, ±8 G

if mag.data_ready()? {
    let [x, y, z] = mag.read_microtesla()?;
}
```

The datasheet gives no self-test limits, so `self_test` returns the per-axis differences
in milligauss and leaves the decision to the application.

## Features

- `async`: adds `qmc5883p_ng::asynch::Qmc5883p` on `embedded-hal-async`.
- `defmt`: derives `defmt::Format` for the public types.

The crates.io name `qmc5883p` belongs to an unrelated driver; this crate is published as
`qmc5883p-ng`.

## License

Copyright © 2026 Markus Mayer

Triple licensed under your choice of either of:

- European Union Public Licence, Version 1.2, ([LICENSE-EUPL](LICENSE-EUPL)
  or https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12)
- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

[crates-image]: https://img.shields.io/crates/v/qmc5883p-ng
[crates-link]: https://crates.io/crates/qmc5883p-ng
[docs-image]: https://docs.rs/qmc5883p-ng/badge.svg
[docs-link]: https://docs.rs/qmc5883p-ng/
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
