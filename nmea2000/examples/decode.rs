use std::time::Duration;

use anyhow::{Context, Result};
use socketcan::{
    CanDataFrame, CanInterface, CanSocket, EmbeddedFrame, ExtendedId, Id, Socket,
    available_interfaces,
};

use nmea2000::{
    Nmea2000,
    packets::{Packet, handshake::ProductInformation},
    util::fixed_string,
};

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

fn main() -> Result<()> {
    let available = available_interfaces()?;
    let ifname = available.first().context("No CAN interfaces.")?;
    let iface = CanInterface::open(ifname)?;
    let socket = CanSocket::open(ifname)?;

    iface.bring_down()?;
    iface.set_bitrate(250_000, None)?;
    iface.bring_up()?;

    let mut nmea2000 = Nmea2000::new();

    loop {
        if let Ok(frame) = socket.read_frame_timeout(Duration::from_millis(100))
            && let Id::Extended(id) = frame.id()
            && let Some(packet) = nmea2000.on_packet(id.as_raw(), pad_data(frame.data()))
        {
            println!("Got: {packet:?}");
            match packet {
                Packet::IsoRequest(packet) => match packet.pgn {
                    0x1F014 => nmea2000.enqueue(Packet::ProductInformation(PRODUCT_INFO), 0xFF),
                    _ => {}
                },
                _ => {}
            }
        };

        for packet in nmea2000.dequeue() {
            println!("Writing: {packet:?}");
            let frame = CanDataFrame::new(
                Id::Extended(ExtendedId::new(packet.id).unwrap()),
                &packet.data,
            );
            socket.write_frame(&frame.unwrap()).unwrap();
        }
    }
}

fn pad_data(data: &[u8]) -> [u8; 8] {
    let mut out = [0; 8];
    for (i, &byte) in data.iter().enumerate() {
        out[i] = byte;
    }
    out
}
