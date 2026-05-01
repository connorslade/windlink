use std::{
    io::{self, Read, Write},
    mem::ManuallyDrop,
    net::TcpListener,
    sync::Arc,
    thread,
};

use anyhow::{Error, Result};
use clone_macro::clone;
use esp_idf_hal::{delay::FreeRtos, io::Write as _, modem::WifiModem, sys::twai_message_t};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    http::{Method, server::EspHttpServer},
    mdns::EspMdns,
    ota::EspOta,
    wifi::{AccessPointConfiguration, BlockingWifi, Configuration, EspWifi},
};
use log::info;
use nmea2000::packets::RawPacket;

use crate::{
    app::App,
    util::{ForceLock, SharedStream},
};

pub struct WirelessClient {
    stream: SharedStream,
}

pub fn init(app: Arc<App>, modem: WifiModem<'static>) -> Result<()> {
    let sysloop = EspSystemEventLoop::take()?;
    let driver = EspWifi::new(modem, sysloop.clone(), Some(app.nvs.clone()))?;
    let mut wifi = ManuallyDrop::new(BlockingWifi::wrap(driver, sysloop)?);

    let config = AccessPointConfiguration {
        ssid: "windlink".try_into().unwrap(),
        ..Default::default()
    };

    wifi.set_configuration(&Configuration::AccessPoint(config))?;
    wifi.start()?;
    wifi.wait_netif_up()?;
    info!("Initialized WiFi");

    let mut mdns = ManuallyDrop::new(EspMdns::take()?);
    mdns.set_hostname("windlink")?;
    mdns.add_service(None, "_http", "_tcp", 80, &[])?;
    mdns.add_service(None, "_windlink", "_tcp", 40, &[])?;

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

        thread::spawn(|| {
            FreeRtos::delay_ms(1000);
            esp_idf_hal::reset::restart();
        });
        Ok(())
    })?;

    http.fn_handler::<Error, _>(
        "/logs",
        Method::Get,
        clone!([app], move |req| {
            let mut resp = req.into_ok_response()?;
            let logs = (app.logs.force_lock().iter())
                .map(|x| x.as_str())
                .intersperse("\n")
                .collect::<String>();
            resp.write_all(&logs.as_bytes())?;
            Ok(())
        }),
    )?;

    let socket = TcpListener::bind("0.0.0.0:40")?;
    thread::spawn(move || {
        for stream in socket.incoming().filter_map(|x| x.ok()) {
            let mut stream = SharedStream::new(stream);

            let mut wireless = app.wireless.force_lock();
            wireless.push(WirelessClient::new(&stream));
            drop(wireless);

            thread::spawn(clone!([app], move || {
                while let Ok((id, data)) = read_api_packet(&mut stream) {
                    app.enqueue_packet(RawPacket::new_raw(id, data));
                }
            }));
        }
    });

    Ok(())
}

impl WirelessClient {
    pub fn new(stream: &SharedStream) -> Self {
        Self {
            stream: stream.clone(),
        }
    }

    fn _write(&mut self, frame: twai_message_t) -> io::Result<()> {
        let bytes = frame.data_length_code as usize;
        self.stream.write_all(&frame.identifier.to_be_bytes())?;
        self.stream.write_all(&[frame.data_length_code])?;
        self.stream.write_all(&frame.data[0..bytes])?;
        self.stream.flush()?;
        Ok(())
    }

    // returns true if socket was closed
    pub fn write(&mut self, frame: twai_message_t) -> bool {
        self._write(frame).is_err()
    }
}

fn read_api_packet(reader: &mut impl Read) -> Result<(u32, [u8; 8])> {
    let mut ident = [0_u8; 4];
    reader.read_exact(&mut ident)?;

    let mut length = [0_u8; 1];
    reader.read_exact(&mut length)?;
    let length = length[0] as usize;

    let mut data = [0_u8; 8];
    reader.read_exact(&mut data[..length])?;

    Ok((u32::from_be_bytes(ident), data))
}
