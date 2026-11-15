//! Decoding of the output registers.

use qmc5883p_registers::Range;

/// A magnetic field vector in raw sensor units.
///
/// The sensitivity depends on the [`Range`] the sample was taken with.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct MagneticField {
    /// The X component.
    pub x: i16,
    /// The Y component.
    pub y: i16,
    /// The Z component.
    pub z: i16,
}

impl MagneticField {
    /// The number of bytes of a vector (three little-endian 16-bit values).
    pub const LEN: usize = 6;

    /// Creates a vector from raw components.
    pub const fn new(x: i16, y: i16, z: i16) -> Self {
        Self { x, y, z }
    }

    /// Decodes registers `XOUT_L` through `ZOUT_H`.
    pub const fn from_le_bytes(bytes: &[u8; Self::LEN]) -> Self {
        Self {
            x: i16::from_le_bytes([bytes[0], bytes[1]]),
            y: i16::from_le_bytes([bytes[2], bytes[3]]),
            z: i16::from_le_bytes([bytes[4], bytes[5]]),
        }
    }

    /// The components in gauss, for a sample taken with the specified range.
    pub fn gauss(&self, range: Range) -> [f32; 3] {
        let lsb_per_gauss = f32::from(range.lsb_per_gauss());
        [self.x, self.y, self.z].map(|v| f32::from(v) / lsb_per_gauss)
    }

    /// The components in microtesla (1 G = 100 µT), for a sample taken with the specified
    /// range.
    pub fn microtesla(&self, range: Range) -> [f32; 3] {
        let lsb_per_gauss = f32::from(range.lsb_per_gauss());
        [self.x, self.y, self.z].map(|v| f32::from(v) * 100.0 / lsb_per_gauss)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoding_and_units() {
        let field = MagneticField::from_le_bytes(&[0xE8, 0x03, 0x18, 0xFC, 0xA6, 0x0E]);
        assert_eq!(field, MagneticField::new(1000, -1000, 3750));
        assert_eq!(field.gauss(Range::Gauss30), [1.0, -1.0, 3.75]);
        assert_eq!(field.microtesla(Range::Gauss30), [100.0, -100.0, 375.0]);
        assert_eq!(
            MagneticField::new(3750, 0, 0).microtesla(Range::Gauss8)[0],
            100.0
        );
        assert_eq!(MagneticField::new(15000, 0, 0).gauss(Range::Gauss2)[0], 1.0);
        assert_eq!(MagneticField::new(2500, 0, 0).gauss(Range::Gauss12)[0], 1.0);
    }
}
