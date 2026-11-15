# Changelog

All notable changes to this project will be documented in this file.
This project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `coordinate-frame` feature with the `SensorFrame` alias (`EastNorthUp`, unverified on
  hardware) for converting readings into other coordinate frames.

## [0.1.0] - 2026-11-15

[0.1.0]: https://github.com/sunsided/qmc5883p/releases/tag/v0.1.0

### Added

- Blocking driver on embedded-hal 1.0 and async driver on embedded-hal-async (`async` feature).
- Chip ID check, soft reset, configuration of mode, output data rate, over sampling,
  range and set/reset mode.
- Data-ready and overflow polling; magnetic field in raw LSB, gauss or microtesla.
- Self-test following the datasheet procedure.
