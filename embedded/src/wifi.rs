use std::{
    io::{self, BufWriter, Write},
    mem::ManuallyDrop,
    net::{TcpListener, TcpStream},
    sync::Arc,
    thread,
};

use anyhow::{Error, Result};
use esp_idf_hal::{delay::FreeRtos, modem::WifiModem, sys::twai_message_t};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    http::{Method, server::EspHttpServer},
    mdns::EspMdns,
    ota::EspOta,
    wifi::{AccessPointConfiguration, AuthMethod, BlockingWifi, Configuration, EspWifi},
};
use log::info;

use crate::{app::App, util::ForceLock};

pub struct WirelessClient {
    stream: BufWriter<TcpStream>,
}

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

    let mut mdns = ManuallyDrop::new(EspMdns::take()?);
    mdns.set_hostname("windlink")?;
    mdns.add_service(None, "_http", "_tcp", 80, &[])?;

    let mut http = ManuallyDrop::new(EspHttpServer::new(&Default::default())?);
    http.fn_handler::<Error, _>("/", Method::Get, |req| {
        let mut resp = req.into_ok_response()?;
        resp.write(include_bytes!("../web/index.html"))?;
        Ok(())
    })?;

    http.fn_handler::<Error, _>("/update", Method::Post, |req| {
        let mut ota = EspOta::new()?;
        ota.factory_reset()?;
        req.into_ok_response()?.flush()?;

        FreeRtos::delay_ms(500);
        esp_idf_hal::reset::restart();
    })?;

    let socket = TcpListener::bind("0.0.0.0:40")?;
    thread::spawn(move || {
        for stream in socket.incoming().filter_map(|x| x.ok()) {
            let mut wireless = app.wireless.force_lock();
            wireless.push(WirelessClient {
                stream: BufWriter::new(stream),
            });
        }
    });

    Ok(())
}

impl WirelessClient {
    fn _write(&mut self, frame: twai_message_t) -> io::Result<()> {
        let bytes = frame.data_length_code as usize;
        self.stream.write_all(&frame.identifier.to_be_bytes())?;
        self.stream.write_all(&[frame.data_length_code])?;
        self.stream.write_all(&frame.data[0..bytes])?;
        Ok(())
    }

    // returns true if socket was closed
    pub fn write(&mut self, frame: twai_message_t) -> bool {
        self._write(frame).is_err()
    }
}
