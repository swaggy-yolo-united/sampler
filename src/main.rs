use esp_idf_svc::hal::gpio::PinDriver;
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::hal::spi::config::{Config, DriverConfig};
use esp_idf_svc::hal::spi::{SpiDriver, SpiSharedDeviceDriver};
use esp_idf_svc::hal::units::Hertz;

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

    let sclk = peripherals.pins.gpio4;

    let sdo = peripherals.pins.gpio5;
    let sdi = Some(peripherals.pins.gpio7);

    let mut reset = PinDriver::output(peripherals.pins.gpio8)?;
    let dreq = PinDriver::input(peripherals.pins.gpio10)?;

    let device = SpiDriver::new(peripherals.spi2, sclk, sdo, sdi, &DriverConfig::default())?;

    let mut cs = PinDriver::output(peripherals.pins.gpio6)?;
    let ctrl_config = Config::default().baudrate(Hertz(250000));
    let ctrl_driver = SpiSharedDeviceDriver::new(&device, &ctrl_config)?;

    let mut card_cs = PinDriver::output(peripherals.pins.gpio0)?;
    let data_config = Config::default().baudrate(Hertz(8000000));
    let data_driver = SpiSharedDeviceDriver::new(&device, &data_config);

    // Setup chip selects
    {
        reset.set_low()?;
        cs.set_high()?;
        card_cs.set_high()?;
    }

    // Reset
    {
        reset.set_low()?;
        reset.set_high()?;
    }

    // Try reading then writing shit
    // {
    //     ctrl_driver.lock(|driver| {
    //         driver.write(&[]).unwrap();
    //         let mut data: &mut [u8] = &mut [];
    //         driver.read(&mut data).unwrap();
    //     });
    // }

    log::info!("Hello, world!");
    Ok(())
}
