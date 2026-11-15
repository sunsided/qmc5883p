//! # QST QMC5883P Magnetometer Registers
//!
//! This crate provides a typed map of the QMC5883P's I²C registers, following
//! datasheet QST-PD-B002-22 Rev E.
//!
//! The output data is split into one register per byte, little-endian (low byte first).
//! The datasheet reads `01H` to `06H` in one go (section 7.5); the register address
//! advances with every byte read.

#![deny(missing_docs)]
#![no_std]
#![forbid(unsafe_code)]
// bitfield-struct generates `with_*_checked` setters that return `Result<_, ()>`.
#![allow(clippy::result_unit_err)]
// Enables the `doc_cfg` feature when the `docsrs` configuration attribute is defined.
#![cfg_attr(docsrs, feature(doc_cfg))]

/// Exports commonly used traits.
pub mod prelude {
    pub use crate::{Register, WritableRegister};
    pub use hardware_registers::i2c::*;
    pub use hardware_registers::register_address::RegisterAddress8;
    pub use hardware_registers::sizes::R1;
    pub use hardware_registers::{
        FromBits, HardwareRegister, ToBits, TryFromBits, WritableHardwareRegister,
    };
}

macro_rules! readable_register {
    ($type:ident, $addr:expr) => {
        impl $crate::Register for $type {}
        impl $crate::prelude::HardwareRegister<$crate::prelude::R1> for $type {}

        impl
            $crate::prelude::I2CRegister<
                $crate::prelude::DeviceAddress7,
                $crate::prelude::RegisterAddress8,
                $crate::prelude::R1,
            > for $type
        {
            type Backing = u8;

            const DEFAULT_DEVICE_ADDRESS: $crate::prelude::DeviceAddress7 =
                $crate::prelude::DeviceAddress7::new($crate::DEFAULT_DEVICE_ADDRESS);
            const REGISTER_ADDRESS: $crate::prelude::RegisterAddress8 =
                $crate::prelude::RegisterAddress8::new(($addr).addr());
        }

        impl $crate::prelude::ToBits for $type {
            type Target = u8;

            #[inline]
            fn to_bits(&self) -> Self::Target {
                (*self).into()
            }
        }

        impl $crate::prelude::FromBits<u8> for $type {
            #[inline]
            fn from_bits(value: u8) -> Self {
                value.into()
            }

            #[inline]
            fn from_bits_ref(value: &u8) -> Self {
                (*value).into()
            }
        }
    };
}

macro_rules! writable_register {
    ($type:ident, $addr:expr) => {
        readable_register!($type, $addr);
        impl $crate::prelude::WritableHardwareRegister<$crate::prelude::R1> for $type {}
        impl $crate::WritableRegister for $type {}
    };
}

mod registers;
mod types;

pub use registers::*;
pub use types::*;

/// The I²C bus address.
///
/// The datasheet (section 5.4) gives `2CH` as the default 7-bit address and refers to the
/// factory for other addresses.
pub const DEFAULT_DEVICE_ADDRESS: u8 = 0x2C;

/// The value of the [`ChipId`] register for a QMC5883P (datasheet section 9.2.1).
pub const QMC5883P_CHIP_ID: u8 = 0x80;

/// The value the datasheet writes to [`AxisSign`] before every setup (section 7).
///
/// The datasheet describes it only as "define the sign for X Y and Z axis".
pub const AXIS_SIGN_DEFAULT: u8 = 0x06;

/// The output code above which [`Status::ovfl`] is set, per axis and sign
/// (datasheet section 9.2.2).
pub const OVERFLOW_LIMIT: i16 = 30_000;

/// A readable magnetometer register.
///
/// Every bit pattern decodes, so reading is infallible.
pub trait Register:
    prelude::ReadableI2CRegister8<prelude::DeviceAddress7> + prelude::FromBits<u8> + From<u8> + Into<u8>
{
}

/// A writable magnetometer register.
pub trait WritableRegister:
    prelude::WritableI2CRegister8<prelude::DeviceAddress7> + Register
{
}
