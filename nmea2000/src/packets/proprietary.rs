use crate::util::bits;

#[derive(Debug)]
pub struct SimnetAp {
    pub address: u8,
    pub proprietary: u8,
    pub command: u8,
    pub event: u8,
}

impl SimnetAp {
    pub const PGN: u32 = 0x1FF22;

    pub fn deserialize(data: &[u8]) -> Self {
        let manufacturer = u16::from_le_bytes([data[0], data[1] & bits(3)]);
        let industry = (data[1] >> 5) & bits(3);
        assert_eq!(manufacturer, 1857);
        assert_eq!(industry, 4);

        Self {
            address: data[2],
            proprietary: data[4],
            command: data[5],
            event: data[6],
        }
    }

    pub fn serialize(&self) -> [u8; 12] {
        let mut out = [0; 12];
        out[..2].copy_from_slice(&u16::to_le_bytes(1857));
        out[1] |= 4 << 5;
        out[2] = self.address;
        out[3] = 0xFF;
        out[4] = self.proprietary;
        out[5] = self.command;
        out[6] = self.event;
        out[7..12].fill(0xFF);
        out
    }
}
