//! # QST QMC5883P behavioral model
//!
//! A register-level model of the QMC5883P magnetometer, used in two places:
//!
//! - as a fake I²C bus ([`FakeI2c`]) in host tests of the `qmc5883p-ng` driver, with a
//!   delay ([`FakeDelay`]) that advances the model's time, and
//! - wrapped into a [Wokwi](https://wokwi.com) custom chip (`sim/wokwi-chip`).
//!
//! The model follows datasheet QST-PD-B002-22 Rev E and implements:
//!
//! - the I²C protocol (register pointer write followed by reads or writes),
//! - suspend, normal (at the output data rate), single and continuous mode,
//! - the field ranges and their sensitivities, with output saturation at -32768 and 32767,
//! - the data-ready (`DRDY`) and overflow (`OVFL`) flags, reset by reading [`Status`],
//! - self-test, which adds [`MODEL_SELF_TEST_MILLIGAUSS`] to the next sample,
//! - soft reset.
//!
//! The field is set in milligauss with [`Qmc5883pModel::set_field`] and converted with the
//! sensitivity of the configured range at every sample.
//!
//! ## Not modelled
//!
//! Over sampling (`OSR1`, `OSR2`), set/reset, noise, offset and the sensitivity decrease at
//! high fields. The bits can be written and read back, but do not change the output.
//!
//! ## Assumptions
//!
//! The datasheet leaves the following open; the model fills the gaps as stated.
//!
//! - **Address auto-increment.** The datasheet reads `01H` to `06H` in one go, but does
//!   not describe the register pointer. The model advances the pointer after every byte
//!   read or written, wrapping from `FFh` to `00h`.
//! - **Undocumented addresses** (`07h`, `08h`, `0Ch` to `28h`, `2Ah` and up) read `0` and
//!   ignore writes.
//! - **Reset values.** Only the chip ID (`80h`) and suspend mode are documented. The model
//!   resets [`Control1`], [`Control2`] and [`AxisSign`] to `00h`.
//! - **Axis sign register (`29h`).** Only the value `06h` is documented, as "define the
//!   sign for X Y and Z axis". The model stores the byte and ignores it: the field passed
//!   to [`Qmc5883pModel::set_field`] is the output for any value.
//! - **Continuous mode rate.** The datasheet gives only the maximum of 1500 Hz. The model
//!   samples every [`CONTINUOUS_PERIOD_US`], whatever the output data rate and over
//!   sampling ratios.
//! - **Single mode timing.** No conversion time is documented. The model samples as soon
//!   as single mode is written, then enters suspend mode.
//! - **Self-test.** No stimulus is documented, and the datasheet computes the difference as
//!   `x1 - x2`, `y2 - y1`, `z2 - z1`. The model adds `-X`, `+Y`, `+Z`
//!   [`MODEL_SELF_TEST_MILLIGAUSS`] to the next continuous-mode sample, so that all three
//!   differences come out positive, then clears `SELF_TEST` and enters suspend mode
//!   (section 6.2.3). Outside continuous mode, the bit is cleared and nothing happens.
//! - **Soft reset** takes no time and has the same effect as power-on reset. `SOFT_RST`
//!   reads back as `0`.
//! - **Mode changes** without suspend mode in between, which the datasheet advises
//!   against, are accepted.
//! - **Address.** The model answers at the documented default address `2Ch` only.
//!
//! [`Status`]: qmc5883p_registers::Status
//! [`Control1`]: qmc5883p_registers::Control1
//! [`Control2`]: qmc5883p_registers::Control2
//! [`AxisSign`]: qmc5883p_registers::AxisSign

#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod bus;

pub use bus::{FakeDelay, FakeI2c};

use qmc5883p_registers::{
    Control1, Control2, DEFAULT_DEVICE_ADDRESS, Mode, OVERFLOW_LIMIT, QMC5883P_CHIP_ID,
    RegisterAddress as Reg, Status,
};

/// The field the model adds while self-test is enabled, in milligauss.
///
/// Applied as `-X`, `+Y`, `+Z`; see the crate's assumptions.
pub const MODEL_SELF_TEST_MILLIGAUSS: i32 = 500;

/// The sample period in continuous mode, in microseconds (1500 Hz, rounded).
pub const CONTINUOUS_PERIOD_US: u32 = 667;

/// Register addresses that accept writes.
const WRITABLE: &[Reg] = &[Reg::CONTROL_1, Reg::CONTROL_2, Reg::AXIS_SIGN];

/// A behavioral model of the QMC5883P.
#[derive(Debug, Clone)]
pub struct Qmc5883pModel {
    regs: [u8; 256],
    pointer: u8,
    /// `true` until the first byte of a write transaction set the register pointer.
    expect_pointer: bool,
    /// The magnetic field seen by the sensor, in milligauss.
    field: [i32; 3],
    /// Time since the last sample in normal or continuous mode.
    elapsed_us: u32,
    /// Converts the nanoseconds of [`fake_bus::Clock`] to the whole microseconds of
    /// [`Self::tick`].
    ticks: fake_bus::Micros,
}

impl Default for Qmc5883pModel {
    fn default() -> Self {
        Self::new()
    }
}

impl Qmc5883pModel {
    /// Creates a model freshly out of power-on reset, in suspend mode.
    pub fn new() -> Self {
        let mut model = Self {
            regs: [0; 256],
            pointer: 0,
            expect_pointer: false,
            field: [0; 3],
            elapsed_us: 0,
            ticks: fake_bus::Micros::new(),
        };

        model.power_on();
        model
    }

    /// Resets all registers, as by a power cycle or [`Control2::soft_rst`].
    pub fn power_on(&mut self) {
        self.regs = [0; 256];
        self.set(Reg::CHIP_ID, QMC5883P_CHIP_ID);
        self.pointer = 0;
        self.expect_pointer = false;
        self.elapsed_us = 0;
    }

    /// The 7-bit I²C address of the model.
    pub fn address(&self) -> u8 {
        DEFAULT_DEVICE_ADDRESS
    }

    /// Reads a register directly, bypassing the bus (and its side effects).
    pub fn register(&self, reg: Reg) -> u8 {
        self.regs[usize::from(reg.addr())]
    }

    /// The current operating mode.
    pub fn mode(&self) -> Mode {
        self.control1().mode()
    }

    /// Sets the magnetic field seen by the sensor, in milligauss.
    ///
    /// The field shows in the output registers with the next sample.
    pub fn set_field(&mut self, x: i32, y: i32, z: i32) {
        self.field = [x, y, z];
    }

    /// Advances time by `us` microseconds. In normal mode, the model samples at the output
    /// data rate; in continuous mode, every [`CONTINUOUS_PERIOD_US`].
    pub fn tick(&mut self, us: u32) {
        let control = self.control1();
        let period_us = match control.mode() {
            Mode::Normal => 1_000_000 / u32::from(control.odr().hz()),
            Mode::Continuous => CONTINUOUS_PERIOD_US,
            Mode::Suspend | Mode::Single => return,
        };

        self.elapsed_us = self.elapsed_us.saturating_add(us);
        while self.elapsed_us >= period_us && self.mode() != Mode::Suspend {
            self.elapsed_us -= period_us;
            self.sample();
        }
    }

    fn control1(&self) -> Control1 {
        Control1::from(self.register(Reg::CONTROL_1))
    }

    fn control2(&self) -> Control2 {
        Control2::from(self.register(Reg::CONTROL_2))
    }

    fn set(&mut self, reg: Reg, value: u8) {
        self.regs[usize::from(reg.addr())] = value;
    }

    /// Takes one measurement and sets `DRDY` (and `OVFL` on overflow).
    fn sample(&mut self) {
        let control = self.control2();
        let self_test = control.self_test();
        let stimulus = if self_test {
            [
                -MODEL_SELF_TEST_MILLIGAUSS,
                MODEL_SELF_TEST_MILLIGAUSS,
                MODEL_SELF_TEST_MILLIGAUSS,
            ]
        } else {
            [0; 3]
        };

        let lsb_per_gauss = i64::from(control.rng().lsb_per_gauss());
        let mut overflow = false;
        for (axis, stimulus) in stimulus.into_iter().enumerate() {
            let milligauss = i64::from(self.field[axis]) + i64::from(stimulus);
            let code = milligauss * lsb_per_gauss / 1000;
            overflow |= code.abs() > i64::from(OVERFLOW_LIMIT);
            let code = code.clamp(i64::from(i16::MIN), i64::from(i16::MAX)) as i16;
            let addr = usize::from(Reg::XOUT_L.addr()) + 2 * axis;
            self.regs[addr..addr + 2].copy_from_slice(&code.to_le_bytes());
        }

        let status = u8::from(Status::from(self.register(Reg::STATUS))) | 0x01;
        self.set(Reg::STATUS, status | (u8::from(overflow) << 1));

        if self_test {
            self.set(Reg::CONTROL_2, control.with_self_test(false).into());
            self.set(
                Reg::CONTROL_1,
                self.control1().with_mode(Mode::Suspend).into(),
            );
        }
    }

    fn write_register(&mut self, reg: Reg, value: u8) {
        match reg {
            Reg::CONTROL_1 => {
                let control = Control1::from(value);
                self.set(reg, value);
                self.elapsed_us = 0;
                if control.mode() == Mode::Single {
                    self.sample();
                    self.set(reg, control.with_mode(Mode::Suspend).into());
                }
            }
            Reg::CONTROL_2 => {
                let control = Control2::from(value);
                if control.soft_rst() {
                    self.power_on();
                } else if control.self_test() && self.mode() != Mode::Continuous {
                    self.set(reg, control.with_self_test(false).into());
                } else {
                    self.set(reg, value);
                }
            }
            _ => self.set(reg, value),
        }
    }

    /// Starts an I²C transaction addressed to the model. Returns `true` to acknowledge.
    ///
    /// `is_read` selects a read (`true`) or write (`false`) transfer.
    pub fn i2c_start(&mut self, is_read: bool) -> bool {
        self.expect_pointer = !is_read;
        true
    }

    /// Writes a byte within an I²C write transfer. Returns `true` to acknowledge.
    ///
    /// The first byte sets the register pointer; subsequent bytes are written to
    /// consecutive registers. Writes to read-only or undocumented registers are ignored.
    pub fn i2c_write(&mut self, byte: u8) -> bool {
        if self.expect_pointer {
            self.pointer = byte;
            self.expect_pointer = false;
        } else {
            let addr = self.pointer;
            self.pointer = self.pointer.wrapping_add(1);
            if let Some(&reg) = WRITABLE.iter().find(|r| r.addr() == addr) {
                self.write_register(reg, byte);
            }
        }

        true
    }

    /// Reads the byte at the register pointer within an I²C read transfer and advances
    /// the pointer.
    ///
    /// Reading [`Status`] resets `DRDY` and `OVFL`.
    pub fn i2c_read(&mut self) -> u8 {
        let addr = self.pointer;
        let value = self.regs[usize::from(addr)];
        self.pointer = self.pointer.wrapping_add(1);
        if addr == Reg::STATUS.addr() {
            self.set(Reg::STATUS, 0);
        }

        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(model: &mut Qmc5883pModel, bytes: &[u8]) -> bool {
        model.i2c_start(false) && bytes.iter().all(|&b| model.i2c_write(b))
    }

    fn read<const N: usize>(model: &mut Qmc5883pModel, reg: u8) -> [u8; N] {
        assert!(write(model, &[reg]));
        assert!(model.i2c_start(true));
        core::array::from_fn(|_| model.i2c_read())
    }

    #[test]
    fn power_on_state() {
        let mut model = Qmc5883pModel::new();
        assert_eq!(read::<1>(&mut model, 0x00), [0x80]);
        assert_eq!(read::<3>(&mut model, 0x09), [0x00, 0x00, 0x00]);
        assert_eq!(model.mode(), Mode::Suspend);
    }

    #[test]
    fn normal_mode_samples_at_odr() {
        let mut model = Qmc5883pModel::new();
        model.set_field(100, -200, 300);
        // Range ±30 G (1 LSB per mG), normal mode at 50 Hz.
        assert!(write(&mut model, &[0x0A, 0x05]));
        model.tick(19_999);
        assert_eq!(read::<1>(&mut model, 0x09), [0x00]);
        model.tick(1);
        assert_eq!(
            read::<6>(&mut model, 0x01),
            [100, 0, 0x38, 0xFF, 0x2C, 0x01]
        );
        assert_eq!(read::<1>(&mut model, 0x09), [0x01]);
        assert_eq!(read::<1>(&mut model, 0x09), [0x00]);
    }

    #[test]
    fn continuous_mode_ignores_odr() {
        let mut model = Qmc5883pModel::new();
        assert!(write(&mut model, &[0x0A, 0xC3]));
        model.tick(CONTINUOUS_PERIOD_US);
        assert_eq!(read::<1>(&mut model, 0x09), [0x01]);
    }

    #[test]
    fn range_sets_sensitivity() {
        let mut model = Qmc5883pModel::new();
        model.set_field(400, -400, 1000);
        // Range ±8 G: 3.75 LSB per mG.
        assert!(write(&mut model, &[0x0B, 0x08]));
        assert!(write(&mut model, &[0x0A, 0x02]));
        let raw = read::<6>(&mut model, 0x01);
        assert_eq!(i16::from_le_bytes([raw[0], raw[1]]), 1500);
        assert_eq!(i16::from_le_bytes([raw[2], raw[3]]), -1500);
        assert_eq!(i16::from_le_bytes([raw[4], raw[5]]), 3750);
    }

    #[test]
    fn single_mode_returns_to_suspend() {
        let mut model = Qmc5883pModel::new();
        model.set_field(1, 2, 3);
        assert!(write(&mut model, &[0x0A, 0x02]));
        assert_eq!(model.mode(), Mode::Suspend);
        assert_eq!(read::<6>(&mut model, 0x01), [1, 0, 2, 0, 3, 0]);
        assert_eq!(read::<1>(&mut model, 0x09), [0x01]);
        model.tick(1_000_000);
        assert_eq!(read::<1>(&mut model, 0x09), [0x00]);
    }

    #[test]
    fn overflow_and_saturation() {
        let mut model = Qmc5883pModel::new();
        // Range ±2 G: 15 LSB per mG. 2100 mG is 31500 LSB, 3000 mG saturates.
        model.set_field(2100, -3000, 0);
        assert!(write(&mut model, &[0x0B, 0x0C]));
        assert!(write(&mut model, &[0x0A, 0x02]));
        let raw = read::<4>(&mut model, 0x01);
        assert_eq!(i16::from_le_bytes([raw[0], raw[1]]), 31500);
        assert_eq!(i16::from_le_bytes([raw[2], raw[3]]), i16::MIN);
        assert_eq!(read::<1>(&mut model, 0x09), [0x03]);
        assert_eq!(read::<1>(&mut model, 0x09), [0x00]);

        model.set_field(2000, 0, 0);
        assert!(write(&mut model, &[0x0A, 0x02]));
        assert_eq!(read::<1>(&mut model, 0x09), [0x01]);
    }

    #[test]
    fn self_test_in_continuous_mode() {
        let mut model = Qmc5883pModel::new();
        model.set_field(10, 10, 10);
        assert!(write(&mut model, &[0x0A, 0x03]));
        assert!(write(&mut model, &[0x0B, 0x40]));
        model.tick(5_000);
        let raw = read::<6>(&mut model, 0x01);
        let st = MODEL_SELF_TEST_MILLIGAUSS as i16;
        assert_eq!(i16::from_le_bytes([raw[0], raw[1]]), 10 - st);
        assert_eq!(i16::from_le_bytes([raw[2], raw[3]]), 10 + st);
        assert_eq!(i16::from_le_bytes([raw[4], raw[5]]), 10 + st);
        assert_eq!(read::<2>(&mut model, 0x0A), [0x00, 0x00]);
    }

    #[test]
    fn self_test_ignored_outside_continuous_mode() {
        let mut model = Qmc5883pModel::new();
        assert!(write(&mut model, &[0x0B, 0x48]));
        assert_eq!(read::<1>(&mut model, 0x0B), [0x08]);
    }

    #[test]
    fn soft_reset_restores_defaults() {
        let mut model = Qmc5883pModel::new();
        assert!(write(&mut model, &[0x29, 0x06]));
        assert!(write(&mut model, &[0x0A, 0xCD, 0x08]));
        assert!(write(&mut model, &[0x0B, 0x80]));
        assert_eq!(read::<2>(&mut model, 0x0A), [0x00, 0x00]);
        assert_eq!(read::<1>(&mut model, 0x29), [0x00]);
        assert_eq!(model.register(Reg::CHIP_ID), QMC5883P_CHIP_ID);
    }

    #[test]
    fn read_only_and_undocumented_registers_ignore_writes() {
        let mut model = Qmc5883pModel::new();
        assert!(write(&mut model, &[0x00, 0x12, 0x34]));
        assert!(write(&mut model, &[0x07, 0x55]));
        assert_eq!(read::<2>(&mut model, 0x00), [0x80, 0x00]);
        assert_eq!(read::<1>(&mut model, 0x07), [0x00]);
    }

    #[test]
    fn axis_sign_is_stored() {
        let mut model = Qmc5883pModel::new();
        assert!(write(&mut model, &[0x29, 0x06]));
        assert_eq!(read::<1>(&mut model, 0x29), [0x06]);
    }
}
