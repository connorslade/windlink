#![feature(mapped_lock_guards)]

use std::sync::Arc;

use anyhow::Result;
use esp_idf_hal::peripherals::Peripherals;
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

    loop {
        std::thread::park()
    }
}
