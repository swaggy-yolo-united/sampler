mod block;

use std::{error::Error, fmt::Display, str};

use embedded_sdmmc::{SdCard, SdCardError, Volume, VolumeIdx, VolumeManager};
use esp_idf_hal::{
    delay::Ets,
    gpio::{Gpio0, Output, OutputPin, PinDriver},
    prelude::Peripherals,
    spi::{
        config::{Config, MODE_0},
        SpiDeviceDriver, SpiDriver, SpiDriverConfig, SpiSharedDeviceDriver, SPI2,
    },
    units::{KiloHertz, MegaHertz},
};
// use esp_idf_svc::hal::spi::{SpiDriver, SpiSharedDeviceDriver};
use esp_idf_svc::hal::units::Hertz;

use crate::block::Clock;

#[derive(Debug)]
struct SdError(pub embedded_sdmmc::Error<SdCardError>);

impl Display for SdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl Error for SdError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

fn open_file<'s, 'p>(
    sd: SdCard<&mut SpiDeviceDriver<'s, &SpiDriver<'s>>, PinDriver<'p, Gpio0, Output>, Ets>,
    file_name: &str,
) -> anyhow::Result<String> {
    let mut volume_mgr = VolumeManager::new(sd, Clock);
    log::debug!("Created Volume Manager");
    let volume = volume_mgr.open_raw_volume(VolumeIdx(0)).map_err(SdError)?;
    log::debug!("Opened Volume {:?}", volume);
    let root_dir = volume_mgr.open_root_dir(volume).map_err(SdError)?;
    let file = volume_mgr
        .open_file_in_dir(root_dir, file_name, embedded_sdmmc::Mode::ReadOnly)
        .map_err(SdError)?;
    log::debug!("Opened file: {}", file_name);
    let mut parts: Vec<u8> = Vec::new();
    while !volume_mgr.file_eof(file).unwrap() {
        let mut buffer = [0u8; 32];
        let num_read = volume_mgr.read(file, &mut buffer).unwrap();
        for b in &buffer[0..num_read] {
            parts.push(*b)
        }
    }

    volume_mgr.close_file(file).map_err(SdError)?;
    volume_mgr.close_dir(root_dir).map_err(SdError)?;

    let string = std::str::from_utf8(parts.as_slice())?.to_string();
    Ok(string)
}

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

    let sclk = peripherals.pins.gpio6;

    let mut test = PinDriver::output(peripherals.pins.gpio9)?;
    test.set_high()?;

    let sdo = peripherals.pins.gpio2;
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

    let mut ctrl_cs = PinDriver::output(peripherals.pins.gpio1)?;
    let ctrl_config = Config::default().baudrate(Hertz(250000));
    let ctrl_driver = SpiSharedDeviceDriver::new(&device, &ctrl_config)?;

    let mut sd_cs = PinDriver::output(peripherals.pins.gpio0)?;
    let sd_config = Config::default()
        .baudrate(KiloHertz(400).into())
        .data_mode(MODE_0);
    let sd_driver = SpiSharedDeviceDriver::new(&device, &sd_config)?;

    // Setup chip selects
    {
        reset.set_low()?;
        ctrl_cs.set_high()?;
        sd_cs.set_high()?;
    }

    // Reset
    {
        reset.set_low()?;
        reset.set_high()?;
    }

    sd_driver.lock(|driver| {
        let sd = embedded_sdmmc::SdCard::new(driver, sd_cs, Ets);
        log::debug!("Card opened ({} bytes)", sd.num_bytes().unwrap());
        match open_file(sd, "test.txt") {
            Ok(text) => log::info!("{}", text),
            Err(err) => log::error!("{:?}", err),
        };
    });

    Ok(())
}
