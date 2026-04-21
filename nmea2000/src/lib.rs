mod header;
pub mod packets;
mod util;

use std::{
    collections::HashMap,
    mem,
    time::{Duration, Instant},
};

pub use header::Header;

use crate::{
    packets::{
        Packet, RawPacket,
        fast::{FastPacket, KNOWN_FAST_PACKETS},
        handshake::AddressClaim,
    },
    util::bits,
};

pub struct Nmea2000 {
    address_claimed: bool,
    last_claim: Option<Instant>,
    address: u8,
    start_address: u8,
    seen_packets: bool,

    fast_packet: HashMap<(u32, u8), FastPacket>,
    queue: Vec<RawPacket>,

    address_claim: AddressClaim,
}

impl Nmea2000 {
    pub fn new() -> Self {
        Self {
            address_claimed: false,
            last_claim: None,
            address: 0,
            start_address: 0,
            seen_packets: false,

            fast_packet: HashMap::new(),
            queue: Vec::new(),

            address_claim: AddressClaim {
                unique_number: 1824692,
                manufacturer_code: 2000,
                device_instance_lower: 0,
                device_instance_upper: 0,
                device_function: 150,
                device_class: 80,
                system_instance: 0,
                arbitrary_address_capable: true,
            },
        }
    }

    pub fn with_address_claim(self, address_claim: AddressClaim) -> Self {
        Self {
            address_claim,
            ..self
        }
    }

    pub fn with_prefered_address(self, address: u8) -> Self {
        Self {
            address,
            start_address: address,
            ..self
        }
    }

    pub fn on_packet(&mut self, id: u32, data: [u8; 8]) -> Option<Packet> {
        self.garbage_collect();
        self.seen_packets = true;
        let header = Header::deserialize(id);

        let packet = if KNOWN_FAST_PACKETS.contains(&header.pgn) {
            let frame = data[0] & bits(5);
            let sequence = (data[0] >> 5) & bits(3);

            let key = (header.pgn, sequence);
            if frame == 0 {
                let mut packet = FastPacket::first(data);
                if let Some(data) = packet.finished() {
                    Packet::deserialize_fast(header.pgn, data)
                } else {
                    self.fast_packet.insert(key, packet);
                    None
                }
            } else if let Some(packet) = self.fast_packet.get_mut(&key)
                && let Some(data) = packet.append(data)
            {
                self.fast_packet.remove(&key);
                Packet::deserialize_fast(header.pgn, data)
            } else {
                None
            }
        } else {
            Packet::deserialize_single(header.pgn, data)
        };

        match &packet {
            Some(Packet::IsoRequest(packet)) if packet.pgn == AddressClaim::PGN => {
                self.queue.push(self.address_claim())
            }
            Some(Packet::AddressClaim(packet)) => {
                if header.source == self.address {
                    if self.address_claim.serialize() < packet.serialize() {
                        self.address_claimed = true;
                        self.queue.push(self.address_claim());
                    } else {
                        self.address_claimed = false;
                        self.last_claim = None;
                        self.address = (self.address + 1) % 0xFC;
                        if self.address == self.start_address {
                            self.address = 0xFE;
                        } else {
                            self.queue.push(self.address_claim());
                        }
                    }
                }
            }
            _ => {}
        }

        packet
    }

    pub fn enqueue(&mut self, packet: Packet) {
        packet.serialize(&mut self.queue);
    }

    pub fn dequeue(&mut self) -> Vec<RawPacket> {
        self.garbage_collect();
        let mut packets = self.flush_queue();
        for packet in packets.iter_mut() {
            packet.overwrite_source(self.address);
        }
        packets
    }

    pub fn flush_queue(&mut self) -> Vec<RawPacket> {
        if self.address == 0xFE {
            self.queue.clear();
            return vec![];
        }

        if !self.address_claimed {
            match self.last_claim {
                Some(instant) => {
                    if instant.elapsed() >= Duration::from_millis(250) {
                        if self.seen_packets {
                            self.address_claimed = true;
                        } else {
                            self.last_claim = None;
                        }
                    } else {
                        return vec![];
                    }
                }
                None => {
                    self.last_claim = Some(Instant::now());
                    return vec![self.address_claim()];
                }
            }
        }

        mem::take(&mut self.queue)
    }

    fn address_claim(&self) -> RawPacket {
        RawPacket::new(
            Header::new(AddressClaim::PGN, 6),
            self.address_claim.serialize(),
        )
    }

    fn garbage_collect(&mut self) {
        self.fast_packet
            .retain(|_k, v| v.started_at.elapsed() < Duration::from_secs(1));
    }
}
