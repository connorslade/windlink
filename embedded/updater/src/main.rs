use std::{mem::ManuallyDrop, thread};

use anyhow::{Error, Result};
use esp_idf_hal::{delay::FreeRtos, peripherals::Peripherals};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    http::{Method, server::EspHttpServer},
    log::EspLogger,
    mdns::EspMdns,
    nvs::EspDefaultNvsPartition,
    ota::EspOta,
    wifi::{AccessPointConfiguration, BlockingWifi, Configuration, EspWifi},
};

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let sysloop = EspSystemEventLoop::take()?;
    let driver = EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs))?;
    let mut wifi = ManuallyDrop::new(BlockingWifi::wrap(driver, sysloop)?);

    let config = AccessPointConfiguration {
        ssid: "windlink".try_into().unwrap(),
        ..Default::default()
    };

    wifi.set_configuration(&Configuration::AccessPoint(config))?;
    wifi.start()?;
    wifi.wait_netif_up()?;

    let mut mdns = ManuallyDrop::new(EspMdns::take()?);
    mdns.set_hostname("windlink")?;
    mdns.add_service(None, "_http", "_tcp", 80, &[])?;

    let mut http = ManuallyDrop::new(EspHttpServer::new(&Default::default())?);
    http.fn_handler::<Error, _>("/", Method::Get, |req| {
        let mut resp = req.into_ok_response()?;
        resp.write(include_bytes!("../web/index.html"))?;
        Ok(())
    })?;

    http.fn_handler::<Error, _>("/flash", Method::Post, |mut req| {
        let mut ota = EspOta::new()?;
        let mut update = ota.initiate_update()?;

        let mut buffer = [0; 512];
        loop {
            let size = req.read(&mut buffer)?;
            if size == 0 {
                break;
            }

            update.write(&buffer[..size])?;
        }

        update.complete()?;
        req.into_ok_response()?.flush()?;

        FreeRtos::delay_ms(1000);
        esp_idf_hal::reset::restart();
    })?;

    loop {
        thread::park();
    }
}
