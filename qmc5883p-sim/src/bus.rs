//! [`fake_bus`] adapters for the [`Qmc5883pModel`].

use crate::Qmc5883pModel;
use core::cell::RefCell;
use fake_bus::{Clock, I2cTarget};

/// An I²C bus with a single [`Qmc5883pModel`] attached.
///
/// The model is shared through a [`RefCell`] so that tests can change the field while a
/// driver owns the bus. For a bus shared with other models, use [`fake_bus::FakeI2c`]
/// with an array of targets.
pub type FakeI2c<'a> = fake_bus::FakeI2c<&'a RefCell<Qmc5883pModel>>;

/// A delay provider that advances the time of a [`Qmc5883pModel`] instead of waiting.
pub type FakeDelay<'a> = fake_bus::FakeDelay<'a, Qmc5883pModel>;

impl I2cTarget for Qmc5883pModel {
    fn address(&self) -> u8 {
        Qmc5883pModel::address(self)
    }

    fn start(&mut self, is_read: bool) -> bool {
        self.i2c_start(is_read)
    }

    fn write(&mut self, byte: u8) -> bool {
        self.i2c_write(byte)
    }

    fn read(&mut self) -> u8 {
        self.i2c_read()
    }
}

impl Clock for Qmc5883pModel {
    fn advance_ns(&mut self, ns: u64) {
        for us in self.ticks.advance_u32(ns) {
            self.tick(us);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fake_bus::embedded_hal::delay::DelayNs;
    use fake_bus::embedded_hal::i2c::{ErrorKind, I2c, NoAcknowledgeSource, Operation};

    #[test]
    fn i2c_write_read() {
        let model = RefCell::new(Qmc5883pModel::new());
        let mut bus = FakeI2c::new(&model);
        let mut id = [0];
        bus.write_read(0x2C, &[0x00], &mut id).unwrap();
        assert_eq!(id, [0x80]);
    }

    #[test]
    fn i2c_wrong_address_nacks() {
        let model = RefCell::new(Qmc5883pModel::new());
        let mut bus = FakeI2c::new(&model);
        assert_eq!(
            bus.write(0x0C, &[0x00]),
            Err(ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address))
        );
    }

    #[test]
    fn i2c_adjacent_writes_form_one_transfer() {
        let model = RefCell::new(Qmc5883pModel::new());
        let mut bus = FakeI2c::new(&model);
        bus.transaction(
            0x2C,
            &mut [Operation::Write(&[0x0A]), Operation::Write(&[0x01, 0x08])],
        )
        .unwrap();

        let mut control = [0; 2];
        bus.write_read(0x2C, &[0x0A], &mut control).unwrap();
        assert_eq!(control, [0x01, 0x08]);
    }

    #[test]
    fn i2c_async_write_read() {
        let model = RefCell::new(Qmc5883pModel::new());
        let mut bus = FakeI2c::new(&model);
        let mut id = [0];
        embassy_futures::block_on(fake_bus::embedded_hal_async::i2c::I2c::write_read(
            &mut bus,
            0x2C,
            &[0x00],
            &mut id,
        ))
        .unwrap();
        assert_eq!(id, [0x80]);
    }

    #[test]
    fn delay_advances_time() {
        let model = RefCell::new(Qmc5883pModel::new());
        // Normal mode at 200 Hz: one sample every 5 ms.
        model.borrow_mut().i2c_start(false);
        model.borrow_mut().i2c_write(0x0A);
        model.borrow_mut().i2c_write(0x0D);
        let mut delay = FakeDelay::new(&model);

        // 5 ms in steps of 999 ns plus one short step: no time may get lost.
        for _ in 0..5_005 {
            delay.delay_ns(999);
        }
        delay.delay_ns(5);
        let status = model
            .borrow()
            .register(qmc5883p_registers::RegisterAddress::STATUS);
        assert_eq!(status & 0x01, 0x01);
    }
}
