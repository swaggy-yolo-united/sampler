mod block;

use embedded_sdmmc::{Volume, VolumeIdx, VolumeManager};
use esp_idf_hal::{
    delay::{Delay, Ets},
    gpio::PinDriver,
    prelude::Peripherals,
    spi::{
        config::{Config, DriverConfig, Mode, MODE_3},
        Spi, SpiBusDriver, SpiDeviceDriver, SpiDriver, SpiDriverConfig, SpiSharedDeviceDriver,
        SpiSingleDeviceDriver, SPI2,
    },
    units::MegaHertz,
};
// use esp_idf_svc::hal::spi::{SpiDriver, SpiSharedDeviceDriver};
use esp_idf_svc::hal::units::Hertz;

use crate::block::Clock;

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

    let device = SpiDriver::new::<SPI2>(
        peripherals.spi2,
        sclk,
        sdo,
        sdi,
        &SpiDriverConfig::default(),
    )?;

    // let mut cs = PinDriver::output(peripherals.pins.gpio6)?;
    // let ctrl_config = Config::default().baudrate(Hertz(250000));
    // let ctrl_driver = SpiSharedDeviceDriver::new(&device, &ctrl_config)?;

    let mut card_cs = PinDriver::output(peripherals.pins.gpio0)?;
    let data_config = Config::default().baudrate(MegaHertz(12).into());
    let data_driver = SpiSharedDeviceDriver::new(&device, &data_config)?;
    // let data_driver = SpiDeviceDriver::new(device, Some(card_cs), &data_config);
    // let data_driver = SpiBusDriver::new(device, &data_config)?;

    // let data_driver = SpiDeviceDriver::new(&device, None, &data_config)?;

    // Setup chip selects
    {
        reset.set_low()?;
        // cs.set_high()?;
        card_cs.set_high()?;
    }

    // Reset
    {
        reset.set_low()?;
        reset.set_high()?;
    }

    log::info!("Locking driver");
    data_driver.lock(|driver| {
        let sd = embedded_sdmmc::SdCard::new(driver, card_cs, Ets);
        log::info!("Card opened ({} bytes)", sd.num_bytes().unwrap());
        let mut volume_mgr = VolumeManager::new(sd, Clock);
        log::info!("Created Volume Manager");
        let volume = volume_mgr.open_raw_volume(VolumeIdx(0)).unwrap();
        log::info!("Opened Volume {:?}", volume);
        let root_dir = volume_mgr.open_root_dir(volume).unwrap();
        let file = volume_mgr
            .open_file_in_dir(root_dir, "track001.mp3", embedded_sdmmc::Mode::ReadOnly)
            .unwrap();
        log::info!("Opened file!");
        // while !volume_mgr.file_eof(file).unwrap() {
        //     let mut buffer = [0u8; 32];
        //     let num_read = volume_mgr.read(file, &mut buffer).unwrap();
        //     for b in &buffer[0..num_read] {
        //         print!("{}", *b as char);
        //     }
        // }

        volume_mgr.close_file(file).unwrap();
        volume_mgr.close_dir(root_dir).unwrap();
    });

    log::info!("Hello, world!");
    Ok(())
}
