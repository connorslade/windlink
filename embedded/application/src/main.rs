#![feature(mapped_lock_guards)]
#![feature(iter_intersperse)]

use std::{sync::Arc, thread};

use anyhow::Result;
use esp_idf_hal::peripherals::Peripherals;

use crate::app::{App, MemoryLogger};

mod app;
mod ble;
mod can;
mod indicator;
mod util;
mod wifi;

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();

    let peripherals = Peripherals::take()?;
    let (wifi_modem, _, ble_modem) = peripherals.modem.split();
    let pins = peripherals.pins;
    let ledc = peripherals.ledc;

    let app = Arc::new(App::new());
    let logger = Box::new(MemoryLogger { app: app.clone() });
    log::set_logger(Box::leak(logger))?;
    log::set_max_level(log::LevelFilter::Info);

    wifi::init(app.clone(), wifi_modem)?;
    ble::init(app.clone(), ble_modem)?;
    can::init(app.clone(), peripherals.can, pins.gpio4, pins.gpio5)?;
    indicator::init(app.clone(), ledc.channel0, ledc.timer0, pins.gpio14)?;

    loop {
        thread::park();
    }
}
