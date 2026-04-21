use crate::util::bits;

/// PGN 129025 - Position, Rapid Update
///
/// In 1×10⁻⁷ deg
#[derive(Debug, Clone)]
pub struct PositionRapidUpdate {
    pub latitude: i32,
    pub longitude: i32,
}

impl PositionRapidUpdate {
    pub const PGN: u32 = 0x1F801;

    pub fn deserialize(data: u64) -> Self {
        Self {
            latitude: (data & bits(32)) as _,
            longitude: (data >> 32 & bits(32)) as _,
        }
    }

    pub fn serialize(&self) -> u64 {
        (self.latitude as u64) | (self.longitude as u64) << 32
    }
}

/// PGN 129026 - COG & SOG, Rapid Update
#[derive(Debug, Clone)]
pub struct CogSogRapidUpdate {
    pub sid: u8,
    pub cog_reference: u8,
    pub cog: u16,
    pub sog: u16,
}

impl CogSogRapidUpdate {
    pub const PGN: u32 = 0x1F802;

    pub fn deserialize(data: u64) -> Self {
        Self {
            sid: (data & bits(8)) as _,
            cog_reference: (data >> 8 & bits(2)) as _,
            cog: (data >> 16 & bits(16)) as _,
            sog: (data >> 32 & bits(16)) as _,
        }
    }

    pub fn serialize(&self) -> u64 {
        self.sid as u64
            | (self.cog_reference as u64) << 8
            | (self.cog as u64) << 16
            | (self.sog as u64) << 32
    }
}

/// PGN 127250 - Vessel Heading
#[derive(Debug, Clone)]
pub struct VesselHeading {
    pub sid: u8,
    pub heading: u16,
    pub deviation: u16,
    pub variation: u16,
    pub reference: u8,
}

impl VesselHeading {
    pub const PGN: u32 = 0x1F112;

    pub fn deserialize(data: u64) -> Self {
        Self {
            sid: (data & bits(8)) as _,
            heading: (data >> 8 & bits(16)) as _,
            deviation: (data >> 24 & bits(16)) as _,
            variation: (data >> 40 & bits(16)) as _,
            reference: (data >> 56 & bits(2)) as _,
        }
    }

    pub fn serialize(&self) -> u64 {
        self.sid as u64
            | (self.heading as u64) << 8
            | (self.deviation as u64) << 24
            | (self.variation as u64) << 40
            | (self.reference as u64) << 56
    }
}

/// PGN 130306 - Wind Data
#[derive(Debug, Clone)]
pub struct WindData {
    pub sid: u8,
    pub wind_speed: u16,
    pub wind_angle: u16,
    pub reference: u8,
}

impl WindData {
    pub const PGN: u32 = 0x1FD02;

    pub fn deserialize(data: u64) -> Self {
        Self {
            sid: (data & bits(8)) as _,
            wind_speed: (data >> 8 & bits(16)) as _,
            wind_angle: (data >> 24 & bits(16)) as _,
            reference: (data >> 40 & bits(3)) as _,
        }
    }

    pub fn serialize(&self) -> u64 {
        self.sid as u64
            | (self.wind_speed as u64) << 8
            | (self.wind_angle as u64) << 24
            | (self.reference as u64) << 40
    }
}
