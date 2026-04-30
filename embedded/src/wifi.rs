use std::{mem::ManuallyDrop, sync::Arc};

use anyhow::{Error, Result};
use esp_idf_hal::{modem::WifiModem, sys::esp_restart};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    http::{Method, server::EspHttpServer},
    mdns::EspMdns,
    ota::EspOta,
    wifi::{AccessPointConfiguration, AuthMethod, BlockingWifi, Configuration, EspWifi},
};
use log::info;

use crate::app::App;

pub fn init(app: Arc<App>, modem: WifiModem<'static>) -> Result<()> {
    let sysloop = EspSystemEventLoop::take()?;
    let driver = EspWifi::new(modem, sysloop.clone(), Some(app.nvs.clone()))?;
    let mut wifi = ManuallyDrop::new(BlockingWifi::wrap(driver, sysloop)?);

    let config = AccessPointConfiguration {
        ssid: "windlink".try_into().unwrap(),
        ssid_hidden: false,
        auth_method: AuthMethod::None,
        ..Default::default()
    };

    wifi.set_configuration(&Configuration::AccessPoint(config))?;
    wifi.start()?;
    wifi.wait_netif_up()?;
    info!("Initialized WiFi");

    let ip_info = wifi.wifi().ap_netif().get_ip_info()?;
    info!("AP: {:?}", ip_info.ip);

    let mut http = ManuallyDrop::new(EspHttpServer::new(&Default::default())?);
    http.fn_handler::<Error, _>("/", Method::Get, |req| {
        let mut resp = req.into_ok_response()?;
        resp.write(include_bytes!("../web/index.html"))?;
        Ok(())
    })?;

    let mut mdns = ManuallyDrop::new(EspMdns::take()?);
    mdns.set_hostname("windlink")?;
    mdns.add_service(None, "_http", "_tcp", 80, &[])?;

    http.fn_handler::<Error, _>("/flash", Method::Post, |mut req| {
        let mut ota = EspOta::new()?;
        let mut update = ota.initiate_update().unwrap();

        let mut skip = 3;
        let mut buffer = [0; 4096];
        loop {
            let mut start = 0;
            let size = req.read(&mut buffer).unwrap();
            if size == 0 {
                info!("done!");
                break;
            }

            if skip == 3 {
                info!("{}", String::from_utf8_lossy(&buffer[..size]));
            }

            while skip > 0 && start != size {
                start = buffer[0..size]
                    .iter()
                    .position(|&x| x == b'\n')
                    .unwrap_or(size);
                skip -= 1;
            }

            if start < size {
                info!("writing {start}..{size}");
                update.write(&buffer[start..size]).unwrap();
            }
        }

        update.complete().unwrap();

        let mut resp = req.into_ok_response().unwrap();
        resp.write(include_bytes!("../web/flash-success.html"))?;
        resp.flush()?;

        unsafe { esp_restart() }
    })?;

    Ok(())
}
