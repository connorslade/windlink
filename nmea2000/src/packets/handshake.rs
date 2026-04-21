use crate::util::bits;

/// PGN 60928 - ISO Address Claim
#[derive(Debug, Clone)]
pub struct AddressClaim {
    pub unique_number: u32,
    pub manufacturer_code: u16,
    pub device_instance_lower: u8,
    pub device_instance_upper: u8,
    pub device_function: u8,
    pub device_class: u8,
    pub system_instance: u8,
    pub arbitrary_address_capable: bool,
}

impl AddressClaim {
    pub const PGN: u32 = 0xEE00;

    pub fn deserialize(data: u64) -> Self {
        Self {
            unique_number: (data & bits(21)) as _,
            manufacturer_code: (data >> 21 & bits(11)) as _,
            device_instance_lower: (data >> 32 & bits(3)) as _,
            device_instance_upper: (data >> 35 & bits(5)) as _,
            device_function: (data >> 40 & bits(8)) as _,
            device_class: (data >> 49 & bits(7)) as _,
            system_instance: (data >> 56 & bits(4)) as _,
            arbitrary_address_capable: (data >> 60 & bits(1)) != 0,
        }
    }

    pub fn serialize(&self) -> u64 {
        (self.unique_number as u64)
            | (self.manufacturer_code as u64) << 21
            | (self.device_instance_lower as u64) << 32
            | (self.device_instance_upper as u64) << 35
            | (self.device_function as u64) << 40
            | (self.device_class as u64) << 49
            | (self.system_instance as u64) << 56
            | (self.arbitrary_address_capable as u64) << 60
    }
}

/// PGN 126996 - Product Information
#[derive(Debug, Clone)]
pub struct ProductInformation {
    pub version: u16,
    pub product_code: u16,
    pub model_id: [u8; 32],
    pub software_version: [u8; 32],
    pub model_version: [u8; 32],
    pub serial_code: [u8; 32],
    pub certification_level: u8,
    pub load_equivalency: u8,
}

impl ProductInformation {
    pub const PGN: u32 = 0x1F014;

    pub fn deserialize(data: &[u8]) -> Self {
        ProductInformation {
            version: u16::from_le_bytes([data[0], data[1]]),
            product_code: u16::from_le_bytes([data[2], data[3]]),
            model_id: data[4..36].try_into().unwrap(),
            software_version: data[36..68].try_into().unwrap(),
            model_version: data[68..100].try_into().unwrap(),
            serial_code: data[100..132].try_into().unwrap(),
            certification_level: data[132],
            load_equivalency: data[133],
        }
    }

    pub fn serialize(&self) -> [u8; 134] {
        let mut out = [0; 134];
        out[0..2].copy_from_slice(&self.version.to_le_bytes());
        out[2..4].copy_from_slice(&self.product_code.to_le_bytes());
        out[4..36].copy_from_slice(&self.model_id);
        out[36..68].copy_from_slice(&self.software_version);
        out[68..100].copy_from_slice(&self.model_version);
        out[100..132].copy_from_slice(&self.serial_code);
        out[132] = self.certification_level;
        out[133] = self.load_equivalency;
        out
    }
}

/// PGN 59904 - ISO Request
#[derive(Debug, Clone)]
pub struct IsoRequest {
    pub pgn: u32,
}

impl IsoRequest {
    pub const PGN: u32 = 0xEA00;

    pub fn deserialize(data: u64) -> Self {
        Self {
            pgn: (data & bits(24)) as _,
        }
    }

    pub fn serialize(&self) -> u64 {
        self.pgn as _
    }
}
