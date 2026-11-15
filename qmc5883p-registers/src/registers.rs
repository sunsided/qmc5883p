//! Magnetometer registers (datasheet section 9).

use crate::types::{DownSamplingRate, Mode, OutputDataRate, OverSampleRatio, Range, SetResetMode};
use bitfield_struct::bitfield;

/// Register addresses.
///
/// See also [`DEFAULT_DEVICE_ADDRESS`](crate::DEFAULT_DEVICE_ADDRESS).
#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum RegisterAddress {
    /// See [`ChipId`]. Read-only.
    CHIP_ID = 0x00,
    /// See [`XoutL`]. Read-only.
    XOUT_L = 0x01,
    /// See [`XoutH`]. Read-only.
    XOUT_H = 0x02,
    /// See [`YoutL`]. Read-only.
    YOUT_L = 0x03,
    /// See [`YoutH`]. Read-only.
    YOUT_H = 0x04,
    /// See [`ZoutL`]. Read-only.
    ZOUT_L = 0x05,
    /// See [`ZoutH`]. Read-only.
    ZOUT_H = 0x06,
    /// See [`Status`]. Read-only.
    STATUS = 0x09,
    /// See [`Control1`]. Read-write.
    CONTROL_1 = 0x0A,
    /// See [`Control2`]. Read-write.
    CONTROL_2 = 0x0B,
    /// See [`AxisSign`]. Written by the datasheet's setup examples only.
    AXIS_SIGN = 0x29,
}

impl RegisterAddress {
    /// Returns the address of a register.
    pub const fn addr(&self) -> u8 {
        *self as u8
    }
}

impl From<RegisterAddress> for u8 {
    fn from(value: RegisterAddress) -> Self {
        value.addr()
    }
}

/// Declares a register that holds one byte of a multi-byte value.
macro_rules! byte_register {
    ($(#[$meta:meta])* $type:ident, $addr:expr, $access:ident, $macro:ident) => {
        $(#[$meta])*
        #[bitfield(u8)]
        #[derive(PartialEq, Eq)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub struct $type {
            /// The register value.
            #[bits(8, access = $access)]
            pub value: u8,
        }

        $macro!($type, $addr);
    };
}

/// [`CHIP_ID`](RegisterAddress::CHIP_ID) (00h): chip identification.
#[bitfield(u8)]
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ChipId {
    /// The chip ID. [`QMC5883P_CHIP_ID`](crate::QMC5883P_CHIP_ID) by default.
    #[bits(8, access = RO, default = crate::QMC5883P_CHIP_ID)]
    pub id: u8,
}

readable_register!(ChipId, RegisterAddress::CHIP_ID);

byte_register!(
    /// [`XOUT_L`](RegisterAddress::XOUT_L) (01h): X output, low byte.
    ///
    /// The outputs are 16-bit two's complement values that saturate at -32768 and 32767.
    /// The sensitivity depends on the [`Range`].
    XoutL, RegisterAddress::XOUT_L, RO, readable_register
);
byte_register!(
    /// [`XOUT_H`](RegisterAddress::XOUT_H) (02h): X output, high byte.
    XoutH, RegisterAddress::XOUT_H, RO, readable_register
);
byte_register!(
    /// [`YOUT_L`](RegisterAddress::YOUT_L) (03h): Y output, low byte.
    YoutL, RegisterAddress::YOUT_L, RO, readable_register
);
byte_register!(
    /// [`YOUT_H`](RegisterAddress::YOUT_H) (04h): Y output, high byte.
    YoutH, RegisterAddress::YOUT_H, RO, readable_register
);
byte_register!(
    /// [`ZOUT_L`](RegisterAddress::ZOUT_L) (05h): Z output, low byte.
    ZoutL, RegisterAddress::ZOUT_L, RO, readable_register
);
byte_register!(
    /// [`ZOUT_H`](RegisterAddress::ZOUT_H) (06h): Z output, high byte.
    ZoutH, RegisterAddress::ZOUT_H, RO, readable_register
);

/// [`STATUS`](RegisterAddress::STATUS) (09h): data status.
///
/// Reading the register resets both flags.
#[bitfield(u8)]
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Status {
    /// Data ready: new data of all three axes is in the output registers.
    #[bits(1, access = RO)]
    pub drdy: bool,

    /// Overflow: an axis output exceeded ±[`OVERFLOW_LIMIT`](crate::OVERFLOW_LIMIT) LSB.
    #[bits(1, access = RO)]
    pub ovfl: bool,

    /// Reserved for factory use.
    #[bits(6)]
    __: u8,
}

readable_register!(Status, RegisterAddress::STATUS);

/// [`CONTROL_1`](RegisterAddress::CONTROL_1) (0Ah): mode, output data rate, over sampling.
///
/// The datasheet asks for suspend mode in between changes among normal, single and
/// continuous mode.
#[bitfield(u8)]
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Control1 {
    /// Mode of operation. Defaults to [`Mode::Suspend`].
    #[bits(2, access = RW, default = Mode::Suspend)]
    pub mode: Mode,

    /// Output data rate in normal mode.
    #[bits(2, access = RW, default = OutputDataRate::Hz10)]
    pub odr: OutputDataRate,

    /// Over sample ratio 1 (filter bandwidth).
    #[bits(2, access = RW, default = OverSampleRatio::Osr8)]
    pub osr1: OverSampleRatio,

    /// Over sample ratio 2 (down sampling rate).
    #[bits(2, access = RW, default = DownSamplingRate::Dsr1)]
    pub osr2: DownSamplingRate,
}

writable_register!(Control1, RegisterAddress::CONTROL_1);

/// [`CONTROL_2`](RegisterAddress::CONTROL_2) (0Bh): set/reset, range, self-test, soft reset.
#[bitfield(u8)]
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Control2 {
    /// Set/reset mode.
    #[bits(2, access = RW, default = SetResetMode::SetAndReset)]
    pub set_reset: SetResetMode,

    /// Field range.
    #[bits(2, access = RW, default = Range::Gauss30)]
    pub rng: Range,

    /// Reserved.
    #[bits(2)]
    __: u8,

    /// Self-test. Clears itself after the data is updated. Only works in continuous mode.
    #[bits(1, access = RW)]
    pub self_test: bool,

    /// Soft reset: restores the default value of all registers.
    #[bits(1, access = RW)]
    pub soft_rst: bool,
}

writable_register!(Control2, RegisterAddress::CONTROL_2);

byte_register!(
    /// [`AXIS_SIGN`](RegisterAddress::AXIS_SIGN) (29h): sign of the X, Y and Z axes.
    ///
    /// Not part of the datasheet's register map (section 9). Its setup and self-test
    /// examples (section 7) write [`AXIS_SIGN_DEFAULT`](crate::AXIS_SIGN_DEFAULT) here
    /// first, "to define the sign for X Y and Z axis". No other value, no bit layout and
    /// no reset value are documented.
    AxisSign, RegisterAddress::AXIS_SIGN, RW, writable_register
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        assert_eq!(u8::from(ChipId::new()), 0x80);
        assert_eq!(u8::from(Control1::new()), 0x00);
        assert_eq!(u8::from(Control2::new()), 0x00);
    }

    /// The register values of the datasheet's application examples (section 7).
    #[test]
    fn datasheet_examples() {
        let normal = Control1::new()
            .with_mode(Mode::Normal)
            .with_odr(OutputDataRate::Hz200)
            .with_osr1(OverSampleRatio::Osr8)
            .with_osr2(DownSamplingRate::Dsr8);
        assert_eq!(u8::from(normal), 0xCD);
        assert_eq!(Control1::from(0xC3).mode(), Mode::Continuous);
        assert_eq!(Control1::from(0x03).osr1(), OverSampleRatio::Osr8);

        let range = Control2::new()
            .with_rng(Range::Gauss8)
            .with_set_reset(SetResetMode::SetAndReset);
        assert_eq!(u8::from(range), 0x08);
        assert_eq!(u8::from(Control2::new().with_self_test(true)), 0x40);
        assert_eq!(u8::from(Control2::new().with_soft_rst(true)), 0x80);
    }

    #[test]
    fn status_layout() {
        let s = Status::from(0b10);
        assert!(s.ovfl() && !s.drdy());
        assert!(Status::from(0b01).drdy());
    }
}
