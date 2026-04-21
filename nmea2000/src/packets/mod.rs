use std::fmt::Debug;

use crate::{
    Header,
    packets::{
        fast::encode_fast_packet,
        handshake::{AddressClaim, IsoRequest, ProductInformation},
        motion::{CogSogRapidUpdate, PositionRapidUpdate, VesselHeading, WindData},
    },
};

pub mod fast;
pub mod handshake;
pub mod motion;

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
                encode_fast_packet(
                    Header::new(ProductInformation::PGN, 6),
                    &packet.serialize(),
                    out,
                );
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
