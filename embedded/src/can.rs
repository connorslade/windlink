use std::{mem, sync::Arc, thread};

use anyhow::Result;
use esp_idf_hal::{
    can::{
        CAN, CanConfig, CanDriver, Flags, Frame,
        config::{Filter, Timing},
    },
    delay::{self, TickType},
    gpio::{InputPin, OutputPin},
    sys::{EspError, TickType_t, twai_message_t},
};
use log::info;
use nmea2000::{
    Nmea2000,
    packets::{Packet, handshake::ProductInformation},
    util::fixed_string,
};

use crate::app::App;

const DELAY: TickType = TickType::new_millis(100);
const PRODUCT_INFO: ProductInformation = ProductInformation {
    version: 00001,
    product_code: 1,
    model_id: fixed_string(b"windlink"),
    software_version: fixed_string(b"v0.1.0"),
    model_version: fixed_string(b"v1"),
    serial_code: fixed_string(b"1234"),
    certification_level: 0,
    load_equivalency: 0,
};

pub fn init(
    app: Arc<App>,
    can: CAN<'static>,
    rx: impl InputPin + 'static,
    tx: impl OutputPin + 'static,
) -> Result<()> {
    let config = CanConfig::new()
        .timing(Timing::B500K) // act gonna kms over this
        .filter(Filter::extended_allow_all());
    let mut can = CanDriver::new(can, tx, rx, &config)?;
    can.start()?;

    let mut nmea2000 = Nmea2000::new();
    thread::spawn(move || {
        loop {
            if let Ok(frame) = can_receive_raw(&can, DELAY.ticks())
                && let Some(packet) = nmea2000.on_packet(frame.identifier, frame.data)
            {
                on_packet(&app, &mut nmea2000, packet);
            }

            // todo: don't dequeue if not going to send
            for packet in nmea2000.dequeue() {
                let frame = Frame::new(packet.id, Flags::Extended.into(), &packet.data).unwrap();
                can.transmit(&frame, delay::BLOCK).unwrap();
            }
        }
    });

    info!("Initialized CAN");
    Ok(())
}

fn on_packet(app: &App, nmea2000: &mut Nmea2000, packet: Packet) {
    match packet {
        Packet::IsoRequest(packet) => match packet.pgn {
            0x1F014 => nmea2000.enqueue(Packet::ProductInformation(PRODUCT_INFO)),
            _ => {}
        },
        Packet::PositionRapidUpdate(packet) => {
            app.position_update(packet.latitude, packet.longitude);
        }
        Packet::CogSogRapidUpdate(packet) => {
            app.speed_update(packet.sog);
        }
        Packet::WindData(packet) => {
            app.wind_update(packet.wind_speed, packet.wind_angle);
        }
        _ => {}
    }
}

fn can_receive_raw(
    can: &CanDriver<'static>,
    timeout: TickType_t,
) -> Result<twai_message_t, EspError> {
    can.receive(timeout)
        .map(|frame| unsafe { mem::transmute::<Frame, twai_message_t>(frame) })
}
