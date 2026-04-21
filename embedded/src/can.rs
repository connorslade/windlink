use std::{mem, sync::Arc, thread};

use anyhow::Result;
use esp_idf_hal::{
    can::{
        CAN, CanConfig, CanDriver, Flags, Frame,
        config::{Filter, Timing},
    },
    delay::{self, TickType},
    gpio::{InputPin, OutputPin},
    sys::{esp_random, twai_message_t},
};
use log::{error, info};
use nmea2000::{
    Header,
    packets::{Packet, handshake::AddressClaim},
};

use crate::app::App;

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

    thread::spawn(move || {
        let frame = address_claim();
        while {
            can.transmit(&frame, delay::BLOCK).unwrap();
            can.receive(TickType::new_millis(1000).ticks()).is_err()
        } {}

        loop {
            match can.receive(delay::BLOCK) {
                Ok(frame) => {
                    let frame = unsafe { mem::transmute::<Frame, twai_message_t>(frame) };

                    let header = Header::deserialize(frame.identifier);
                    let packet = Packet::deserialize_single(header.pgn, frame.data);
                    let Some(packet) = packet else { continue };
                    info!("{packet:?}");
                    on_packet(&app, packet);
                }
                Err(err) => error!("CAN receive error: {err}"),
            }
        }
    });

    info!("Initialized CAN");
    Ok(())
}

fn on_packet(app: &App, packet: Packet) {
    match packet {
        Packet::IsoRequest(packet) => {
            info!("Request for PGN {}", packet.pgn);
        }
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

fn address_claim() -> Frame {
    let header = Header::new(AddressClaim::PGN, 6, 11);
    let frame = AddressClaim {
        unique_number: unsafe { esp_random() } & 0x1FFFFF,
        manufacturer_code: 2000,
        device_instance_lower: 0,
        device_instance_upper: 0,
        device_function: 150,
        device_class: 80,
        system_instance: 0,
        arbitrary_address_capable: false,
    };

    Frame::new(
        header.serialize(),
        Flags::Extended.into(),
        &frame.serialize().to_le_bytes(),
    )
    .unwrap()
}
