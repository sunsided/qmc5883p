//! Value types used in register fields.

/// Declares a two-bit field type with `into_bits`/`from_bits` for bitfield-struct.
macro_rules! two_bit_enum {
    (
        $(#[$meta:meta])*
        $name:ident { $($(#[$vmeta:meta])* $variant:ident = $bits:literal),+ $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Copy, Clone, PartialEq, Eq)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        #[repr(u8)]
        pub enum $name {
            $($(#[$vmeta])* $variant = $bits),+
        }

        impl $name {
            /// Converts the value into its register representation.
            pub const fn into_bits(self) -> u8 {
                self as u8
            }

            /// Converts the two lowest bits of a register value.
            pub const fn from_bits(value: u8) -> Self {
                match value & 0b11 {
                    $($bits => Self::$variant,)+
                    _ => unreachable!(),
                }
            }
        }
    };
}

two_bit_enum!(
    /// The operating mode, as set in [`Control1::mode`](crate::Control1) (datasheet section 6).
    Mode {
        /// Suspend mode: no measurement, registers kept, I²C active. The power-on default.
        Suspend = 0b00,
        /// Normal mode: measures periodically at the output data rate.
        Normal = 0b01,
        /// Single mode: measures once, then enters suspend mode.
        Single = 0b10,
        /// Continuous mode: measures all the time without sleep, at the maximum rate.
        Continuous = 0b11,
    }
);

two_bit_enum!(
    /// The output data rate in normal mode, as set in [`Control1::odr`](crate::Control1).
    OutputDataRate {
        /// 10 Hz.
        Hz10 = 0b00,
        /// 50 Hz.
        Hz50 = 0b01,
        /// 100 Hz.
        Hz100 = 0b10,
        /// 200 Hz.
        Hz200 = 0b11,
    }
);

impl OutputDataRate {
    /// Returns the output data rate in Hz.
    pub const fn hz(self) -> u8 {
        match self {
            Self::Hz10 => 10,
            Self::Hz50 => 50,
            Self::Hz100 => 100,
            Self::Hz200 => 200,
        }
    }
}

two_bit_enum!(
    /// Over sample ratio 1, as set in [`Control1::osr1`](crate::Control1).
    ///
    /// Controls the bandwidth of an internal digital filter: a larger ratio gives a smaller
    /// bandwidth, less noise and higher power consumption.
    OverSampleRatio {
        /// 8.
        Osr8 = 0b00,
        /// 4.
        Osr4 = 0b01,
        /// 2.
        Osr2 = 0b10,
        /// 1.
        Osr1 = 0b11,
    }
);

two_bit_enum!(
    /// Over sample ratio 2 (down sampling rate), as set in [`Control1::osr2`](crate::Control1).
    ///
    /// The depth of a second filter for better noise performance.
    DownSamplingRate {
        /// 1.
        Dsr1 = 0b00,
        /// 2.
        Dsr2 = 0b01,
        /// 4.
        Dsr4 = 0b10,
        /// 8.
        Dsr8 = 0b11,
    }
);

two_bit_enum!(
    /// The field range, as set in [`Control2::rng`](crate::Control2).
    Range {
        /// ±30 gauss, 1000 LSB/G.
        Gauss30 = 0b00,
        /// ±12 gauss, 2500 LSB/G.
        Gauss12 = 0b01,
        /// ±8 gauss, 3750 LSB/G.
        Gauss8 = 0b10,
        /// ±2 gauss, 15000 LSB/G.
        Gauss2 = 0b11,
    }
);

impl Range {
    /// The sensitivity in LSB per gauss (datasheet table 2).
    pub const fn lsb_per_gauss(self) -> u16 {
        match self {
            Self::Gauss30 => 1000,
            Self::Gauss12 => 2500,
            Self::Gauss8 => 3750,
            Self::Gauss2 => 15000,
        }
    }
}

/// The set/reset mode, as set in [`Control2::set_reset`](crate::Control2).
///
/// With [`SetOnly`](Self::SetOnly) or [`Off`](Self::Off), the offset is not renewed
/// during measuring.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum SetResetMode {
    /// Set and reset on.
    SetAndReset = 0b00,
    /// Set only on.
    SetOnly = 0b01,
    /// Set and reset off.
    ///
    /// The datasheet assigns both `10` and `11` to this mode.
    Off = 0b10,
}

impl SetResetMode {
    /// Converts the value into its register representation.
    pub const fn into_bits(self) -> u8 {
        self as u8
    }

    /// Converts the two lowest bits of a register value into a set/reset mode.
    pub const fn from_bits(value: u8) -> Self {
        match value & 0b11 {
            0b00 => Self::SetAndReset,
            0b01 => Self::SetOnly,
            _ => Self::Off,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        for bits in 0..4 {
            assert_eq!(Mode::from_bits(bits).into_bits(), bits);
            assert_eq!(OutputDataRate::from_bits(bits).into_bits(), bits);
            assert_eq!(OverSampleRatio::from_bits(bits).into_bits(), bits);
            assert_eq!(DownSamplingRate::from_bits(bits).into_bits(), bits);
            assert_eq!(Range::from_bits(bits).into_bits(), bits);
        }

        assert_eq!(SetResetMode::from_bits(0b11), SetResetMode::Off);
        assert_eq!(SetResetMode::from_bits(0b01), SetResetMode::SetOnly);
    }

    #[test]
    fn values() {
        assert_eq!(OutputDataRate::Hz50.hz(), 50);
        assert_eq!(Range::Gauss8.lsb_per_gauss(), 3750);
    }
}
