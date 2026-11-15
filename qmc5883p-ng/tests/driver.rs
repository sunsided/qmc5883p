//! Driver tests against the behavioral model.

use core::cell::RefCell;
use qmc5883p_ng::registers::{Control1, Control2, RegisterAddress};
use qmc5883p_ng::{
    Config, DownSamplingRate, MagneticField, Mode, OutputDataRate, OverSampleRatio, Qmc5883p,
    Range, SetResetMode,
};
use qmc5883p_sim::{FakeDelay, FakeI2c, MODEL_SELF_TEST_MILLIGAUSS, Qmc5883pModel};

#[test]
fn identifies_chip() {
    let model = RefCell::new(Qmc5883pModel::new());
    let mut mag = Qmc5883p::new(FakeI2c::new(&model));
    assert_eq!(mag.chip_id().unwrap(), 0x80);
    assert!(mag.identify().unwrap());
}

#[test]
fn wrong_address_fails() {
    use embedded_hal::i2c::{ErrorKind, NoAcknowledgeSource};
    let model = RefCell::new(Qmc5883pModel::new());
    let mut mag = Qmc5883p::with_address(FakeI2c::new(&model), 0x0C);
    assert_eq!(
        mag.identify(),
        Err(ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address))
    );
    let _bus = mag.release();
}

#[test]
fn init_resets_and_sets_axis_sign() {
    let model = RefCell::new(Qmc5883pModel::new());
    let mut mag = Qmc5883p::new(FakeI2c::new(&model));
    mag.configure(Config::default()).unwrap();
    mag.init(&mut FakeDelay::new(&model)).unwrap();
    let model = model.borrow();
    assert_eq!(model.register(RegisterAddress::AXIS_SIGN), 0x06);
    assert_eq!(model.register(RegisterAddress::CONTROL_1), 0x00);
    assert_eq!(model.register(RegisterAddress::CONTROL_2), 0x00);
    assert_eq!(model.mode(), Mode::Suspend);
}

#[test]
fn configure_writes_both_control_registers() {
    let model = RefCell::new(Qmc5883pModel::new());
    let mut mag = Qmc5883p::new(FakeI2c::new(&model));
    // The datasheet's normal mode example (section 7.1).
    mag.configure(Config {
        mode: Mode::Normal,
        odr: OutputDataRate::Hz200,
        osr1: OverSampleRatio::Osr8,
        osr2: DownSamplingRate::Dsr8,
        range: Range::Gauss8,
        set_reset: SetResetMode::SetAndReset,
    })
    .unwrap();
    assert_eq!(model.borrow().register(RegisterAddress::CONTROL_1), 0xCD);
    assert_eq!(model.borrow().register(RegisterAddress::CONTROL_2), 0x08);
    assert_eq!(mag.range().unwrap(), Range::Gauss8);
}

#[test]
fn normal_mode_polls_data_ready() {
    let model = RefCell::new(Qmc5883pModel::new());
    model.borrow_mut().set_field(400, -250, 1000);
    let mut mag = Qmc5883p::new(FakeI2c::new(&model));
    mag.configure(Config::default()).unwrap();

    assert!(!mag.data_ready().unwrap());
    model.borrow_mut().tick(20_000);
    assert!(mag.data_ready().unwrap());
    assert!(!mag.data_ready().unwrap());
    assert_eq!(
        mag.read_magnetic_field().unwrap(),
        MagneticField::new(1500, -937, 3750)
    );
    let [x, _, z] = mag.read_microtesla().unwrap();
    assert_eq!((x, z), (40.0, 100.0));
}

#[test]
fn single_measurement() {
    let model = RefCell::new(Qmc5883pModel::new());
    model.borrow_mut().set_field(-1, 2, -3);
    let mut mag = Qmc5883p::new(FakeI2c::new(&model));
    mag.set_mode(Mode::Single).unwrap();
    assert_eq!(model.borrow().mode(), Mode::Suspend);
    assert!(mag.data_ready().unwrap());
    assert_eq!(
        mag.read_magnetic_field().unwrap(),
        MagneticField::new(-1, 2, -3)
    );
}

#[test]
fn set_mode_keeps_configuration() {
    let model = RefCell::new(Qmc5883pModel::new());
    let mut mag = Qmc5883p::new(FakeI2c::new(&model));
    mag.configure(Config::default()).unwrap();
    mag.set_mode(Mode::Continuous).unwrap();
    let control = mag.read_register::<Control1>().unwrap();
    assert_eq!(control.mode(), Mode::Continuous);
    assert_eq!(control.odr(), OutputDataRate::Hz50);
    mag.set_mode(Mode::Suspend).unwrap();
    assert_eq!(model.borrow().mode(), Mode::Suspend);
}

#[test]
fn overflow_flag() {
    let model = RefCell::new(Qmc5883pModel::new());
    model.borrow_mut().set_field(0, 0, 2500);
    let mut mag = Qmc5883p::new(FakeI2c::new(&model));
    mag.configure(Config {
        mode: Mode::Single,
        range: Range::Gauss2,
        ..Config::default()
    })
    .unwrap();
    let status = mag.status().unwrap();
    assert!(status.drdy() && status.ovfl());
    assert_eq!(mag.read_magnetic_field().unwrap().z, i16::MAX);
    assert!(!mag.status().unwrap().ovfl());
}

#[test]
fn range_is_read_back_once() {
    let model = RefCell::new(Qmc5883pModel::new());
    model.borrow_mut().set_field(100, 0, 0);
    let mut mag = Qmc5883p::new(FakeI2c::new(&model));
    mag.configure(Config {
        mode: Mode::Single,
        range: Range::Gauss12,
        ..Config::default()
    })
    .unwrap();

    // A new driver instance (e.g. after an MCU reset) reads the range from the device.
    let mut mag = Qmc5883p::new(mag.release());
    assert_eq!(mag.read_microtesla().unwrap()[0], 10.0);

    mag.modify_register(|r: Control2| r.with_rng(Range::Gauss30))
        .unwrap();
    assert_eq!(mag.range().unwrap(), Range::Gauss30);

    mag.soft_reset(&mut FakeDelay::new(&model)).unwrap();
    assert_eq!(mag.range().unwrap(), Range::Gauss30);
}

#[test]
fn self_test_and_restore() {
    let model = RefCell::new(Qmc5883pModel::new());
    model.borrow_mut().set_field(300, -200, 100);
    let mut mag = Qmc5883p::new(FakeI2c::new(&model));
    mag.configure(Config::default()).unwrap();
    let config = [RegisterAddress::CONTROL_1, RegisterAddress::CONTROL_2];
    let before = config.map(|r| model.borrow().register(r));

    let result = mag.self_test(&mut FakeDelay::new(&model)).unwrap();
    assert_eq!(
        result.difference_milligauss,
        [MODEL_SELF_TEST_MILLIGAUSS; 3]
    );
    assert_eq!(model.borrow().register(RegisterAddress::AXIS_SIGN), 0x06);

    let after = config.map(|r| model.borrow().register(r));
    assert_eq!(before, after);
    assert_eq!(mag.range().unwrap(), Range::Gauss8);
    assert!(!mag.data_ready().unwrap());
}

#[test]
fn self_test_times_out_without_data() {
    use qmc5883p_ng::SelfTestError;
    struct NoDelay;
    impl embedded_hal::delay::DelayNs for NoDelay {
        fn delay_ns(&mut self, _: u32) {}
    }

    let model = RefCell::new(Qmc5883pModel::new());
    let mut mag = Qmc5883p::new(FakeI2c::new(&model));
    assert_eq!(mag.self_test(&mut NoDelay), Err(SelfTestError::Timeout));
    assert_eq!(model.borrow().mode(), Mode::Suspend);
}

#[cfg(feature = "async")]
#[test]
fn async_driver() {
    use embassy_futures::block_on;
    let model = RefCell::new(Qmc5883pModel::new());
    model.borrow_mut().set_field(100, 200, 300);
    let mut mag = qmc5883p_ng::asynch::Qmc5883p::new(FakeI2c::new(&model));
    block_on(async {
        assert!(mag.identify().await.unwrap());
        mag.init(&mut FakeDelay::new(&model)).await.unwrap();
        mag.configure(Config {
            range: Range::Gauss30,
            ..Config::default()
        })
        .await
        .unwrap();
        model.borrow_mut().tick(20_000);
        assert!(mag.data_ready().await.unwrap());
        assert_eq!(mag.read_microtesla().await.unwrap(), [10.0, 20.0, 30.0]);

        let result = mag.self_test(&mut FakeDelay::new(&model)).await.unwrap();
        assert_eq!(
            result.difference_milligauss,
            [MODEL_SELF_TEST_MILLIGAUSS; 3]
        );
        mag.set_mode(Mode::Single).await.unwrap();
        mag.modify_register(|r: Control2| r.with_rng(Range::Gauss2))
            .await
            .unwrap();
        assert_eq!(mag.range().await.unwrap(), Range::Gauss2);
        assert_eq!(model.borrow().mode(), Mode::Suspend);
        let _bus = mag.release();
    });
}
