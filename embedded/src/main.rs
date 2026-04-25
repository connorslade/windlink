#![feature(mapped_lock_guards)]

use std::sync::Arc;

use anyhow::Result;
use esp_idf_hal::{
    delay::FreeRtos,
    ledc::{LedcDriver, LedcTimerDriver, config::TimerConfig},
    peripherals::Peripherals,
    units::KiloHertz,
};
use esp_idf_svc::log::EspLogger;

use crate::app::App;

mod app;
mod ble;
mod can;

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let pins = peripherals.pins;

    let app = Arc::new(App::default());

    ble::init(app.clone(), peripherals.modem)?;
    can::init(app.clone(), peripherals.can, pins.gpio4, pins.gpio5)?;

    let mut driver = LedcDriver::new(
        peripherals.ledc.channel0,
        &LedcTimerDriver::new(
            peripherals.ledc.timer0,
            &TimerConfig::new().frequency(KiloHertz(25).into()),
        )?,
        pins.gpio20,
    )?;

    loop {
        driver.fade_with_time(driver.get_max_duty() / 4, 500, true)?;
        driver.fade_with_time(0, 500, true)?;
        FreeRtos::delay_ms(1000);
    }
}
