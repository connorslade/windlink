use std::{mem, time::Instant};

use crate::{
    Header,
    packets::{RawPacket, handshake::ProductInformation},
};

pub const KNOWN_FAST_PACKETS: &[u32] = &[ProductInformation::PGN];

pub struct FastPacket {
    pub total_length: usize,
    pub data: Vec<u8>,
    pub started_at: Instant,
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

pub fn encode_fast_packet(header: Header, data: &[u8], out: &mut Vec<RawPacket>) {
    let len = data.len() as u8;
    let get = |i: usize| data.get(i).copied().unwrap_or_default();
    out.push(RawPacket::new_bytes(
        header,
        [0, len, get(0), get(1), get(2), get(3), get(4), get(5)],
    ));

    let mut i = 6;
    let mut sequence = 1;
    while i < data.len() {
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
