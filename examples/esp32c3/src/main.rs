//! Reads a QMC5883P magnetometer and prints field and heading over the serial console.
//!
//! Wiring: SDA = GPIO4, SCL = GPIO5. The QMC5883P has no interrupt pin, so the firmware
//! polls the data-ready flag.

#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::delay::Delay;
use esp_hal::i2c::master::{Config as I2cConfig, I2c};
use esp_hal::main;
use esp_hal::time::Rate;
use esp_println::println;
use qmc5883p_ng::{Config, Qmc5883p};

esp_bootloader_esp_idf::esp_app_desc!();

/// The heading in degrees (0–360), with X pointing north and the sensor held flat.
fn heading([x, y, _]: [f32; 3]) -> f32 {
    let degrees = libm::atan2f(y, x).to_degrees();
    if degrees < 0.0 {
        degrees + 360.0
    } else {
        degrees
    }
}

#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let mut delay = Delay::new();

    let i2c = I2c::new(
        peripherals.I2C0,
        I2cConfig::default().with_frequency(Rate::from_khz(400)),
    )
    .expect("valid I2C config")
    .with_sda(peripherals.GPIO4)
    .with_scl(peripherals.GPIO5);

    // Pass the address with `Qmc5883p::with_address` if the board uses another one.
    let mut mag = Qmc5883p::new(i2c);
    let chip_id = mag.chip_id().expect("device responds");
    if chip_id != qmc5883p_ng::QMC5883P_CHIP_ID {
        panic!("unexpected chip ID 0x{chip_id:02X}");
    }

    println!("QMC5883P ok chip_id=0x{chip_id:02X}");

    mag.init(&mut delay).unwrap();
    let st = mag.self_test(&mut delay).expect("self-test runs");
    let [dx, dy, dz] = st.difference_milligauss;
    println!("self-test dx={dx} dy={dy} dz={dz} mG");

    // Normal mode at 50 Hz, OSR1 = 8, ±8 G.
    mag.configure(Config::default()).unwrap();
    println!("running odr=50Hz range=8G");

    let mut last = None;
    loop {
        if mag.data_ready().unwrap() {
            let field = mag.read_microtesla().unwrap();
            if last != Some(field) {
                let [x, y, z] = field;
                println!(
                    "mag x={x:.1} y={y:.1} z={z:.1} uT heading={:.0}",
                    heading(field)
                );
                last = Some(field);
            }
        }

        delay.delay_millis(5);
    }
}
