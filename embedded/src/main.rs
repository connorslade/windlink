#![feature(mapped_lock_guards)]

use std::{sync::Arc, thread};

use anyhow::Result;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::log::EspLogger;

use crate::app::App;

mod app;
mod ble;
mod can;
mod indicator;
mod util;
mod wifi;

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let (wifi_modem, _, ble_modem) = peripherals.modem.split();
    let pins = peripherals.pins;
    let ledc = peripherals.ledc;

    let app = Arc::new(App::new());

    wifi::init(app.clone(), wifi_modem)?;
    ble::init(app.clone(), ble_modem)?;
    can::init(app.clone(), peripherals.can, pins.gpio4, pins.gpio5)?;
    indicator::init(app.clone(), ledc.channel0, ledc.timer0, pins.gpio20)?;

    loop {
        thread::park();
    }
}
