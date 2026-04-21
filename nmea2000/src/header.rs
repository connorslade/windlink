#[derive(Debug, Clone, Copy)]
pub struct Header {
    pub priority: u8,
    pub source: u8,
    pub pgn: u32,
}

impl Header {
    pub fn new(pgn: u32, priority: u8) -> Self {
        Self {
            priority,
            source: 0,
            pgn,
        }
    }

    pub fn deserialize(id: u32) -> Self {
        let id = id & 0x1FFFFFFF;
        Self {
            priority: ((id >> 26) & 0x7) as u8,
            source: (id & 0xFF) as u8,
            pgn: {
                let data_page = (id >> 24) & 1;
                let pdu_format = ((id >> 16) & 0xFF) as u8;
                let pdu_specific = ((id >> 8) & 0xFF) as u8;
                if pdu_format < 0xF0 {
                    (data_page << 16) | ((pdu_format as u32) << 8)
                } else {
                    (data_page << 16) | ((pdu_format as u32) << 8) | (pdu_specific as u32)
                }
            },
        }
    }

    pub fn serialize(&self) -> u32 {
        let data_page = (self.pgn >> 16) & 1;
        let pdu_format = (self.pgn >> 8) & 0xFF;
        let pdu_specific = if pdu_format < 0xF0 {
            0xFF
        } else {
            self.pgn & 0xFF
        };

        ((self.priority as u32 & 0x7) << 26)
            | (data_page << 24)
            | (pdu_format << 16)
            | (pdu_specific << 8)
            | (self.source as u32 & 0xFF)
    }
}
