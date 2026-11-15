//! # QST QMC5883P magnetometer driver
//!
//! > A platform-agnostic driver to interface with the QST QMC5883P 3-axis magnetometer
//! > over I²C.
//!
//! This driver was built using [`embedded-hal`] 1.0 and [`qmc5883p-registers`]. Enable the
//! `async` feature for an equivalent driver on [`embedded-hal-async`] in `asynch`.
//!
//! The driver takes the bus by value. To share the bus with other devices, pass a
//! bus handle such as `embedded_hal_bus::i2c::RefCellDevice`, or `&mut` to a bus.
//!
//! ## Example
//!
//! ```
//! # use core::cell::RefCell;
//! # use qmc5883p_sim::{FakeDelay, FakeI2c, Qmc5883pModel};
//! use qmc5883p_ng::{Config, Mode, OutputDataRate, Qmc5883p, Range};
//! # let model = RefCell::new(Qmc5883pModel::new());
//! # model.borrow_mut().set_field(200, 0, -300);
//! # let mut delay = FakeDelay::new(&model);
//! # let i2c = FakeI2c::new(&model);
//!
//! let mut mag = Qmc5883p::new(i2c);
//! assert!(mag.identify()?);
//! mag.init(&mut delay)?;
//! mag.configure(Config {
//!     mode: Mode::Normal,
//!     odr: OutputDataRate::Hz50,
//!     range: Range::Gauss8,
//!     ..Config::default()
//! })?;
//! # model.borrow_mut().tick(20_000);
//!
//! if mag.data_ready()? {
//!     let [x, y, z] = mag.read_microtesla()?;
//!     assert_eq!((x, y, z), (20.0, 0.0, -30.0));
//! }
//! # Ok::<(), embedded_hal::i2c::ErrorKind>(())
//! ```
//!
//! ## Range tracking
//!
//! The sensitivity depends on the field range. The driver remembers the range it writes to
//! `CONTROL_2`, and reads `CONTROL_2` once before the first conversion otherwise, so a
//! device that kept its configuration across a microcontroller reset is converted
//! correctly too.
//!
//! [`embedded-hal`]: https://crates.io/crates/embedded-hal
//! [`embedded-hal-async`]: https://crates.io/crates/embedded-hal-async
//! [`qmc5883p-registers`]: https://crates.io/crates/qmc5883p-registers

#![deny(missing_docs)]
#![deny(warnings)]
#![no_std]
#![forbid(unsafe_code)]
// Enables the `doc_cfg` feature when the `docsrs` configuration attribute is defined.
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "async")]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
pub mod asynch;
mod measurement;
mod self_test;

pub use measurement::MagneticField;
pub use qmc5883p_registers as registers;
pub use qmc5883p_registers::{
    AXIS_SIGN_DEFAULT, DEFAULT_DEVICE_ADDRESS, DownSamplingRate, Mode, OutputDataRate,
    OverSampleRatio, QMC5883P_CHIP_ID, Range, SetResetMode,
};
pub use self_test::{SELF_TEST_WAIT_MS, SelfTest, SelfTestError};

use embedded_hal::delay::DelayNs;
use embedded_hal::i2c::I2c;
use qmc5883p_registers::prelude::*;
use qmc5883p_registers::{AxisSign, ChipId, Control1, Control2, RegisterAddress, Status};

/// The wait after a soft reset, in microseconds.
///
/// The datasheet gives no soft reset time. This is the power-on reset time (`PORT`,
/// datasheet table 7), after which the device accepts I²C commands.
pub const SOFT_RESET_TIME_US: u32 = 250;

/// The measurement configuration.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Config {
    /// The operating mode.
    pub mode: Mode,
    /// The output data rate in normal mode.
    pub odr: OutputDataRate,
    /// Over sample ratio 1 (filter bandwidth).
    pub osr1: OverSampleRatio,
    /// Over sample ratio 2 (down sampling rate).
    pub osr2: DownSamplingRate,
    /// The field range.
    pub range: Range,
    /// The set/reset mode.
    pub set_reset: SetResetMode,
}

impl Default for Config {
    /// Normal mode at 50 Hz, `OSR1` = 8, `OSR2` = 1, ±8 G with set and reset on.
    ///
    /// Range and set/reset follow the datasheet's setup examples (section 7).
    fn default() -> Self {
        Self {
            mode: Mode::Normal,
            odr: OutputDataRate::Hz50,
            osr1: OverSampleRatio::Osr8,
            osr2: DownSamplingRate::Dsr1,
            range: Range::Gauss8,
            set_reset: SetResetMode::SetAndReset,
        }
    }
}

impl Config {
    pub(crate) fn control1(&self) -> Control1 {
        Control1::new()
            .with_mode(self.mode)
            .with_odr(self.odr)
            .with_osr1(self.osr1)
            .with_osr2(self.osr2)
    }

    pub(crate) fn control2(&self) -> Control2 {
        Control2::new()
            .with_rng(self.range)
            .with_set_reset(self.set_reset)
    }
}

/// Updates the tracked range after a register write.
///
/// Returns the new state: `Some(range)` if known, `None` if it must be read back.
pub(crate) fn track_range(range: Option<Range>, address: u8, bits: u8) -> Option<Range> {
    if address == RegisterAddress::CONTROL_2.addr() {
        let control = Control2::from(bits);
        if control.soft_rst() {
            Some(Control2::new().rng())
        } else {
            Some(control.rng())
        }
    } else {
        range
    }
}

/// QMC5883P driver.
#[derive(Debug)]
pub struct Qmc5883p<I2C> {
    i2c: I2C,
    address: u8,
    /// The field range, if known.
    range: Option<Range>,
}

impl<I2C> Qmc5883p<I2C> {
    /// Creates a driver for a device at [`DEFAULT_DEVICE_ADDRESS`]. Performs no bus access.
    pub fn new(i2c: I2C) -> Self {
        Self::with_address(i2c, DEFAULT_DEVICE_ADDRESS)
    }

    /// Creates a driver for a device at the specified 7-bit address. Performs no bus access.
    pub fn with_address(i2c: I2C, address: u8) -> Self {
        Self {
            i2c,
            address,
            range: None,
        }
    }

    /// Consumes the driver and returns the I²C bus.
    pub fn release(self) -> I2C {
        self.i2c
    }
}

impl<I2C> Qmc5883p<I2C>
where
    I2C: I2c,
{
    /// Reads the chip ID.
    pub fn chip_id(&mut self) -> Result<u8, I2C::Error> {
        Ok(self.read_register::<ChipId>()?.id())
    }

    /// Checks whether the chip ID matches [`QMC5883P_CHIP_ID`].
    pub fn identify(&mut self) -> Result<bool, I2C::Error> {
        Ok(self.chip_id()? == QMC5883P_CHIP_ID)
    }

    /// Resets the device and writes [`AXIS_SIGN_DEFAULT`] to the axis sign register, as
    /// the datasheet does before every setup. Leaves the device in suspend mode.
    pub fn init<D>(&mut self, delay: &mut D) -> Result<(), I2C::Error>
    where
        D: DelayNs,
    {
        self.soft_reset(delay)?;
        self.write_register(AxisSign::new().with_value(AXIS_SIGN_DEFAULT))
    }

    /// Restores the default value of all registers and waits [`SOFT_RESET_TIME_US`].
    pub fn soft_reset<D>(&mut self, delay: &mut D) -> Result<(), I2C::Error>
    where
        D: DelayNs,
    {
        self.write_register(Control2::new().with_soft_rst(true))?;
        delay.delay_us(SOFT_RESET_TIME_US);
        Ok(())
    }

    /// Writes the configuration.
    ///
    /// Enters suspend mode first, as the datasheet asks for between mode changes.
    pub fn configure(&mut self, config: Config) -> Result<(), I2C::Error> {
        let control1 = config.control1();
        self.write_register(control1.with_mode(Mode::Suspend))?;
        self.write_register(config.control2())?;
        self.write_register(control1)
    }

    /// Changes the operating mode, through suspend mode. Keeps the rest of the configuration.
    ///
    /// [`Mode::Single`] triggers one measurement; the device then returns to suspend mode.
    pub fn set_mode(&mut self, mode: Mode) -> Result<(), I2C::Error> {
        let control1 = self.read_register::<Control1>()?;
        self.write_register(control1.with_mode(Mode::Suspend))?;
        if mode != Mode::Suspend {
            self.write_register(control1.with_mode(mode))?;
        }

        Ok(())
    }

    /// Reads the status register. Resets the data-ready and overflow flags.
    pub fn status(&mut self) -> Result<Status, I2C::Error> {
        self.read_register()
    }

    /// Whether new data of all three axes is available. Resets the overflow flag as well.
    pub fn data_ready(&mut self) -> Result<bool, I2C::Error> {
        Ok(self.status()?.drdy())
    }

    /// Reads the X, Y and Z outputs in a single burst.
    pub fn read_magnetic_field(&mut self) -> Result<MagneticField, I2C::Error> {
        let mut bytes = [0; MagneticField::LEN];
        self.i2c
            .write_read(self.address, &[RegisterAddress::XOUT_L.addr()], &mut bytes)?;
        Ok(MagneticField::from_le_bytes(&bytes))
    }

    /// Reads the X, Y and Z outputs and converts them to microtesla with the current range.
    pub fn read_microtesla(&mut self) -> Result<[f32; 3], I2C::Error> {
        let range = self.range()?;
        Ok(self.read_magnetic_field()?.microtesla(range))
    }

    /// The field range, read from the device if unknown.
    pub fn range(&mut self) -> Result<Range, I2C::Error> {
        match self.range {
            Some(range) => Ok(range),
            None => {
                let range = self.read_register::<Control2>()?.rng();
                self.range = Some(range);
                Ok(range)
            }
        }
    }

    /// Runs the self-test of datasheet section 7.3 and restores the configuration afterwards.
    ///
    /// Writes [`AXIS_SIGN_DEFAULT`] to the axis sign register, as the datasheet does, and
    /// reads the status register at the end. The magnetic field must stay constant during
    /// the test.
    pub fn self_test<D>(&mut self, delay: &mut D) -> Result<SelfTest, SelfTestError<I2C::Error>>
    where
        D: DelayNs,
    {
        let saved1 = self.read_register::<Control1>()?;
        let saved2 = self.read_register::<Control2>()?;

        let result = self.run_self_test(saved1, delay);

        // Restore, even if the test failed.
        self.write_register(saved1.with_mode(Mode::Suspend))?;
        self.write_register(saved2)?;
        self.write_register(saved1)?;
        // Clears `DRDY` of the self-test sample, so that it is not mistaken for new data.
        self.status()?;
        result
    }

    fn run_self_test<D>(
        &mut self,
        saved1: Control1,
        delay: &mut D,
    ) -> Result<SelfTest, SelfTestError<I2C::Error>>
    where
        D: DelayNs,
    {
        self.write_register(saved1.with_mode(Mode::Suspend))?;
        self.write_register(AxisSign::new().with_value(AXIS_SIGN_DEFAULT))?;
        self.write_register(self_test::control2())?;
        self.write_register(self_test::control1())?;

        let mut ready = false;
        for _ in 0..self_test::MAX_POLLS {
            ready = self.data_ready()?;
            if ready {
                break;
            }

            delay.delay_ms(self_test::POLL_MS);
        }

        if !ready {
            return Err(SelfTestError::Timeout);
        }

        let before = self.read_magnetic_field()?;
        self.write_register(self_test::control2().with_self_test(true))?;
        delay.delay_ms(SELF_TEST_WAIT_MS);
        let with = self.read_magnetic_field()?;
        Ok(SelfTest::from_samples(before, with))
    }

    /// Reads a single register.
    pub fn read_register<R>(&mut self) -> Result<R, I2C::Error>
    where
        R: Register,
    {
        let mut buffer = [0];
        self.i2c
            .write_read(self.address, &[*R::REGISTER_ADDRESS], &mut buffer)?;
        Ok(R::from_bits(buffer[0]))
    }

    /// Writes a single register.
    pub fn write_register<R>(&mut self, register: R) -> Result<(), I2C::Error>
    where
        R: WritableRegister,
    {
        let bits = register.to_bits();
        self.i2c
            .write(self.address, &[*R::REGISTER_ADDRESS, bits])?;
        self.range = track_range(self.range, *R::REGISTER_ADDRESS, bits);
        Ok(())
    }

    /// Reads, modifies and writes back a single register.
    pub fn modify_register<R, F>(&mut self, f: F) -> Result<(), I2C::Error>
    where
        R: WritableRegister,
        F: FnOnce(R) -> R,
    {
        let register = self.read_register::<R>()?;
        self.write_register(f(register))
    }
}
