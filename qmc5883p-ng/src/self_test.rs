//! Self-test procedure, shared by the blocking and async drivers.
//!
//! The procedure follows datasheet section 7.3: sample in continuous mode, enable
//! `SELF_TEST`, wait 5 ms, sample again and compute the difference. The datasheet gives
//! no pass/fail limits, so the driver reports the difference only.

use crate::MagneticField;
use qmc5883p_registers::{Control1, Control2, Mode};

/// The wait after enabling self-test, in milliseconds (datasheet section 7.3, step 6).
pub const SELF_TEST_WAIT_MS: u32 = 5;

/// The polling interval while waiting for data, in milliseconds.
pub(crate) const POLL_MS: u32 = 1;

/// The number of polls before giving up on a sample.
pub(crate) const MAX_POLLS: u32 = 100;

/// An error during the self-test.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SelfTestError<E> {
    /// A bus error.
    Bus(E),
    /// The device did not signal new data in time.
    Timeout,
}

impl<E> From<E> for SelfTestError<E> {
    fn from(error: E) -> Self {
        Self::Bus(error)
    }
}

/// The result of a self-test.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SelfTest {
    /// The difference per axis as the datasheet computes it: `x1 - x2`, `y2 - y1`,
    /// `z2 - z1`, where `1` is the sample before and `2` the sample with self-test.
    ///
    /// Both samples are taken in the ±30 G range (1000 LSB/G), so one LSB is one
    /// milligauss. Compare against limits of your own; the datasheet gives none.
    pub difference_milligauss: [i32; 3],
}

impl SelfTest {
    pub(crate) fn from_samples(before: MagneticField, with: MagneticField) -> Self {
        Self {
            difference_milligauss: [
                i32::from(before.x) - i32::from(with.x),
                i32::from(with.y) - i32::from(before.y),
                i32::from(with.z) - i32::from(before.z),
            ],
        }
    }
}

/// Continuous mode with the remaining `CONTROL_1` bits at zero, as in datasheet step 2.
pub(crate) fn control1() -> Control1 {
    Control1::new().with_mode(Mode::Continuous)
}

/// `CONTROL_2` before the first sample: ±30 G, set and reset on.
///
/// The datasheet keeps the previous range for the first sample, but step 5 writes `40h`,
/// which selects ±30 G for the second. Selecting it up front makes both samples comparable.
pub(crate) fn control2() -> Control2 {
    Control2::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn datasheet_values() {
        assert_eq!(u8::from(control1()), 0x03);
        assert_eq!(u8::from(control2().with_self_test(true)), 0x40);
    }

    #[test]
    fn difference_signs() {
        let result = SelfTest::from_samples(
            MagneticField::new(10, 10, 10),
            MagneticField::new(-5, 30, 40),
        );
        assert_eq!(result.difference_milligauss, [15, 20, 30]);
    }
}
