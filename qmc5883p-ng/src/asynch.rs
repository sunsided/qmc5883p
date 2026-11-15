//! Async driver on `embedded-hal-async`.
//!
//! Mirrors the blocking [`Qmc5883p`](crate::Qmc5883p).

use crate::{
    AXIS_SIGN_DEFAULT, Config, DEFAULT_DEVICE_ADDRESS, MagneticField, Mode, QMC5883P_CHIP_ID,
    Range, SELF_TEST_WAIT_MS, SOFT_RESET_TIME_US, SelfTest, SelfTestError, self_test, track_range,
};

use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::i2c::I2c;
use qmc5883p_registers::prelude::*;
use qmc5883p_registers::{AxisSign, ChipId, Control1, Control2, RegisterAddress, Status};

/// Async QMC5883P driver.
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
    pub async fn chip_id(&mut self) -> Result<u8, I2C::Error> {
        Ok(self.read_register::<ChipId>().await?.id())
    }

    /// Checks whether the chip ID matches [`QMC5883P_CHIP_ID`].
    pub async fn identify(&mut self) -> Result<bool, I2C::Error> {
        Ok(self.chip_id().await? == QMC5883P_CHIP_ID)
    }

    /// Resets the device and writes [`AXIS_SIGN_DEFAULT`] to the axis sign register, as
    /// the datasheet does before every setup. Leaves the device in suspend mode.
    pub async fn init<D>(&mut self, delay: &mut D) -> Result<(), I2C::Error>
    where
        D: DelayNs,
    {
        self.soft_reset(delay).await?;
        self.write_register(AxisSign::new().with_value(AXIS_SIGN_DEFAULT))
            .await
    }

    /// Restores the default value of all registers and waits [`SOFT_RESET_TIME_US`].
    pub async fn soft_reset<D>(&mut self, delay: &mut D) -> Result<(), I2C::Error>
    where
        D: DelayNs,
    {
        self.write_register(Control2::new().with_soft_rst(true))
            .await?;
        delay.delay_us(SOFT_RESET_TIME_US).await;
        Ok(())
    }

    /// Writes the configuration.
    ///
    /// Enters suspend mode first, as the datasheet asks for between mode changes.
    pub async fn configure(&mut self, config: Config) -> Result<(), I2C::Error> {
        let control1 = config.control1();
        self.write_register(control1.with_mode(Mode::Suspend))
            .await?;
        self.write_register(config.control2()).await?;
        self.write_register(control1).await
    }

    /// Changes the operating mode, through suspend mode. Keeps the rest of the configuration.
    ///
    /// [`Mode::Single`] triggers one measurement; the device then returns to suspend mode.
    pub async fn set_mode(&mut self, mode: Mode) -> Result<(), I2C::Error> {
        let control1 = self.read_register::<Control1>().await?;
        self.write_register(control1.with_mode(Mode::Suspend))
            .await?;
        if mode != Mode::Suspend {
            self.write_register(control1.with_mode(mode)).await?;
        }

        Ok(())
    }

    /// Reads the status register. Resets the data-ready and overflow flags.
    pub async fn status(&mut self) -> Result<Status, I2C::Error> {
        self.read_register().await
    }

    /// Whether new data of all three axes is available. Resets the overflow flag as well.
    pub async fn data_ready(&mut self) -> Result<bool, I2C::Error> {
        Ok(self.status().await?.drdy())
    }

    /// Reads the X, Y and Z outputs in a single burst.
    pub async fn read_magnetic_field(&mut self) -> Result<MagneticField, I2C::Error> {
        let mut bytes = [0; MagneticField::LEN];
        self.i2c
            .write_read(self.address, &[RegisterAddress::XOUT_L.addr()], &mut bytes)
            .await?;
        Ok(MagneticField::from_le_bytes(&bytes))
    }

    /// Reads the X, Y and Z outputs and converts them to microtesla with the current range.
    pub async fn read_microtesla(&mut self) -> Result<[f32; 3], I2C::Error> {
        let range = self.range().await?;
        Ok(self.read_magnetic_field().await?.microtesla(range))
    }

    /// The field range, read from the device if unknown.
    pub async fn range(&mut self) -> Result<Range, I2C::Error> {
        match self.range {
            Some(range) => Ok(range),
            None => {
                let range = self.read_register::<Control2>().await?.rng();
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
    pub async fn self_test<D>(
        &mut self,
        delay: &mut D,
    ) -> Result<SelfTest, SelfTestError<I2C::Error>>
    where
        D: DelayNs,
    {
        let saved1 = self.read_register::<Control1>().await?;
        let saved2 = self.read_register::<Control2>().await?;

        let result = self.run_self_test(saved1, delay).await;

        // Restore, even if the test failed.
        self.write_register(saved1.with_mode(Mode::Suspend)).await?;
        self.write_register(saved2).await?;
        self.write_register(saved1).await?;
        // Clears `DRDY` of the self-test sample, so that it is not mistaken for new data.
        self.status().await?;
        result
    }

    async fn run_self_test<D>(
        &mut self,
        saved1: Control1,
        delay: &mut D,
    ) -> Result<SelfTest, SelfTestError<I2C::Error>>
    where
        D: DelayNs,
    {
        self.write_register(saved1.with_mode(Mode::Suspend)).await?;
        self.write_register(AxisSign::new().with_value(AXIS_SIGN_DEFAULT))
            .await?;
        self.write_register(self_test::control2()).await?;
        self.write_register(self_test::control1()).await?;

        let mut ready = false;
        for _ in 0..self_test::MAX_POLLS {
            ready = self.data_ready().await?;
            if ready {
                break;
            }

            delay.delay_ms(self_test::POLL_MS).await;
        }

        if !ready {
            return Err(SelfTestError::Timeout);
        }

        let before = self.read_magnetic_field().await?;
        self.write_register(self_test::control2().with_self_test(true))
            .await?;
        delay.delay_ms(SELF_TEST_WAIT_MS).await;
        let with = self.read_magnetic_field().await?;
        Ok(SelfTest::from_samples(before, with))
    }

    /// Reads a single register.
    pub async fn read_register<R>(&mut self) -> Result<R, I2C::Error>
    where
        R: Register,
    {
        let mut buffer = [0];
        self.i2c
            .write_read(self.address, &[*R::REGISTER_ADDRESS], &mut buffer)
            .await?;
        Ok(R::from_bits(buffer[0]))
    }

    /// Writes a single register.
    pub async fn write_register<R>(&mut self, register: R) -> Result<(), I2C::Error>
    where
        R: WritableRegister,
    {
        let bits = register.to_bits();
        self.i2c
            .write(self.address, &[*R::REGISTER_ADDRESS, bits])
            .await?;
        self.range = track_range(self.range, *R::REGISTER_ADDRESS, bits);
        Ok(())
    }

    /// Reads, modifies and writes back a single register.
    pub async fn modify_register<R, F>(&mut self, f: F) -> Result<(), I2C::Error>
    where
        R: WritableRegister,
        F: FnOnce(R) -> R,
    {
        let register = self.read_register::<R>().await?;
        self.write_register(f(register)).await
    }
}
