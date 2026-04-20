#![feature(mapped_lock_guards)]

use std::sync::Arc;

use anyhow::Result;
use esp_idf_hal::{delay::FreeRtos, peripherals::Peripherals};
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

    let mut speed = 0;
    let mut angle = 0;
    loop {
        app.speed_update(speed);
        speed += 1;
        FreeRtos::delay_ms(500);
        app.wind_update(0, angle);
        angle += 1745;
        FreeRtos::delay_ms(500);
        // std::thread::park()
    }
}
