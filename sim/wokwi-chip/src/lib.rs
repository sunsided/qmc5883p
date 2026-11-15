//! Wokwi custom chip wrapping the `qmc5883p-sim` behavioral model.
//!
//! Pins: `VCC`, `GND`, `SDA`, `SCL` (I²C at `0x2C`). The QMC5883P has no interrupt pin.
//!
//! Controls (also settable as attributes and via `set-control` in scenarios):
//! `fieldX`, `fieldY`, `fieldZ` in microtesla. They are sampled every [`TICK_US`], which
//! is also the time step of the model.

use qmc5883p_sim::Qmc5883pModel;
use std::cell::RefCell;
use std::ffi::{CString, c_void};
use wokwi_chip_ll::*;

/// The model's time step and control sampling period, in microseconds.
const TICK_US: u32 = 1_000;

struct Chip {
    model: Qmc5883pModel,
    field: [AttrId; 3],
}

thread_local! {
    static CHIP: RefCell<Option<Chip>> = const { RefCell::new(None) };
}

fn with_chip<R>(f: impl FnOnce(&mut Chip) -> R) -> R {
    CHIP.with(|chip| f(chip.borrow_mut().as_mut().expect("chipInit not called")))
}

fn cstr(s: &str) -> CString {
    CString::new(s).expect("no interior NUL")
}

fn debug(message: &str) {
    let message = cstr(message);
    // SAFETY: `message` is a valid NUL-terminated string for the duration of the call.
    unsafe { debugPrint(message.as_ptr()) };
}

/// Converts microtesla to milligauss (1 µT = 10 mG), rounding to the nearest milligauss.
fn microtesla_to_milligauss(microtesla: f32) -> i32 {
    (microtesla * 10.0).round() as i32
}

impl Chip {
    fn tick(&mut self) {
        // SAFETY: Attribute IDs were obtained from the simulator in `chipInit`.
        // `attrRead` returns `u32`, which cannot hold negative slider values.
        let [x, y, z] = unsafe { self.field.map(|attr| attrReadFloat(attr) as f32) };

        self.model.set_field(
            microtesla_to_milligauss(x),
            microtesla_to_milligauss(y),
            microtesla_to_milligauss(z),
        );
        self.model.tick(TICK_US);
    }
}

extern "C" fn on_connect(_: *mut c_void, _address: u32, is_read: bool) -> bool {
    with_chip(|chip| chip.model.i2c_start(is_read))
}

extern "C" fn on_read(_: *mut c_void) -> u8 {
    with_chip(|chip| chip.model.i2c_read())
}

extern "C" fn on_write(_: *mut c_void, data: u8) -> bool {
    with_chip(|chip| chip.model.i2c_write(data))
}

extern "C" fn on_disconnect(_: *mut c_void) {}

extern "C" fn on_tick(_: *mut c_void) {
    with_chip(Chip::tick);
}

/// Entry point called by the simulator.
#[unsafe(no_mangle)]
pub extern "C" fn chipInit() {
    // SAFETY: All pointers passed to the simulator are valid for the duration of each
    // call; configs are copied by the simulator. Init functions are only called here.
    unsafe {
        let sda = pinInit(cstr("SDA").as_ptr(), INPUT);
        let scl = pinInit(cstr("SCL").as_ptr(), INPUT);

        let model = Qmc5883pModel::new();
        i2cInit(&I2CConfig {
            user_data: std::ptr::null(),
            address: u32::from(model.address()),
            scl,
            sda,
            connect: on_connect,
            read: on_read,
            write: on_write,
            disconnect: on_disconnect,
        });

        let tick_timer = timerInit(&TimerConfig {
            user_data: std::ptr::null(),
            callback: on_tick,
        });

        let chip = Chip {
            model,
            field: ["fieldX", "fieldY", "fieldZ"].map(|id| attrInit(cstr(id).as_ptr(), 0.0)),
        };

        CHIP.with(|c| *c.borrow_mut() = Some(chip));

        timerStart(tick_timer, TICK_US, true);
    }

    debug("qmc5883p: chip initialized");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions() {
        assert_eq!(microtesla_to_milligauss(30.0), 300);
        assert_eq!(microtesla_to_milligauss(-0.25), -3);
    }
}
