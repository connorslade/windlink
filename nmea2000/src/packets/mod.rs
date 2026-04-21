use std::{fmt::Debug, mem, time::Instant};

use crate::{
    Header,
    packets::{
        handshake::{AddressClaim, IsoRequest, ProductInformation},
        motion::{CogSogRapidUpdate, PositionRapidUpdate, VesselHeading, WindData},
    },
};

pub mod handshake;
pub mod motion;

pub const KNOWN_FAST_PACKETS: &[u32] = &[ProductInformation::PGN];

#[derive(Debug)]
pub enum Packet {
    IsoRequest(IsoRequest),
    AddressClaim(AddressClaim),
    ProductInformation(ProductInformation),
    PositionRapidUpdate(PositionRapidUpdate),
    CogSogRapidUpdate(CogSogRapidUpdate),
    VesselHeading(VesselHeading),
    WindData(WindData),
}

#[derive(Debug)]
pub struct RawPacket {
    pub id: u32,
    pub data: [u8; 8],
}

pub struct FastPacket {
    pub total_length: usize,
    pub data: Vec<u8>,
    pub started_at: Instant,
}

macro_rules! parse_packet {
    ($pgn:expr, $data:expr, [$($ident:ident),*]) => {
        Some(match $pgn {
            $($ident::PGN => Self::$ident($ident::deserialize($data)),)*
            _ => return None,
        })
    };
}

impl Packet {
    pub fn deserialize_single(pgn: u32, data: [u8; 8]) -> Option<Self> {
        let data = u64::from_le_bytes(data);
        parse_packet!(
            pgn,
            data,
            [
                IsoRequest,
                AddressClaim,
                PositionRapidUpdate,
                CogSogRapidUpdate,
                VesselHeading,
                WindData
            ]
        )
    }

    pub fn deserialize_fast(pgn: u32, data: Vec<u8>) -> Option<Self> {
        parse_packet!(pgn, &data, [ProductInformation])
    }

    pub fn serialize(&self, out: &mut Vec<RawPacket>) {
        match self {
            Packet::IsoRequest(packet) => out.push(RawPacket::new(
                Header::new(IsoRequest::PGN, 6),
                packet.serialize(),
            )),
            Packet::ProductInformation(packet) => {
                let header = Header::new(ProductInformation::PGN, 6);
                let bytes = packet.serialize();

                let get = |i: usize| bytes.get(i).copied().unwrap_or_default();
                out.push(RawPacket::new_bytes(
                    header,
                    [
                        0,
                        bytes.len() as u8,
                        get(0),
                        get(1),
                        get(2),
                        get(3),
                        get(4),
                        get(5),
                    ],
                ));

                let mut i = 6;
                let mut sequence = 1;
                while i < bytes.len() {
                    out.push(RawPacket::new_bytes(
                        header,
                        [
                            sequence,
                            get(i + 0),
                            get(i + 1),
                            get(i + 2),
                            get(i + 3),
                            get(i + 4),
                            get(i + 5),
                            get(i + 6),
                        ],
                    ));
                    sequence += 1;
                    i += 7;
                }
            }
            _ => {}
        }
    }
}

impl RawPacket {
    pub fn new(header: Header, data: u64) -> Self {
        Self {
            id: header.serialize(),
            data: data.to_le_bytes(),
        }
    }

    pub fn new_bytes(header: Header, data: [u8; 8]) -> Self {
        Self {
            id: header.serialize(),
            data,
        }
    }

    pub fn overwrite_source(&mut self, source: u8) {
        self.id = self.id & 0xFFFFFF00 | source as u32;
    }
}

impl FastPacket {
    pub fn first(packet: [u8; 8]) -> Self {
        let total_length = packet[1] as usize;
        let mut data = Vec::with_capacity(total_length);
        data.extend_from_slice(&packet[2..2 + total_length.min(6)]);

        Self {
            total_length,
            data,
            started_at: Instant::now(),
        }
    }

    pub fn append(&mut self, data: [u8; 8]) -> Option<Vec<u8>> {
        let expecting = (self.total_length - self.data.len()).min(7);
        self.data.extend_from_slice(&data[1..1 + expecting]);
        (self.data.len() == self.total_length).then(|| mem::take(&mut self.data))
    }

    pub fn finished(&mut self) -> Option<Vec<u8>> {
        (self.data.len() == self.total_length).then(|| mem::take(&mut self.data))
    }
}
