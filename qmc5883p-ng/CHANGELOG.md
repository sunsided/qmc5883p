# Changelog

All notable changes to this project will be documented in this file.
This project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Blocking driver on embedded-hal 1.0 and async driver on embedded-hal-async (`async` feature).
- Chip ID check, soft reset, configuration of mode, output data rate, over sampling,
  range and set/reset mode.
- Data-ready and overflow polling; magnetic field in raw LSB, gauss or microtesla.
- Self-test following the datasheet procedure.
