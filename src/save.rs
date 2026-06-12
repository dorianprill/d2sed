pub struct BitWriter {
    bytes: Vec<u8>,
    bit_offset: usize,
}

impl BitWriter {
    pub fn new() -> Self {
        Self {
            bytes: Vec::new(),
            bit_offset: 0,
        }
    }

    pub fn write_bits(&mut self, value: u32, count: usize) {
        for index in 0..count {
            if self.bit_offset / 8 == self.bytes.len() {
                self.bytes.push(0);
            }
            let bit = ((value >> index) & 1) as u8;
            self.bytes[self.bit_offset / 8] |= bit << (self.bit_offset % 8);
            self.bit_offset += 1;
        }
    }

    pub fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

pub fn calculate_checksum(bytes: &[u8]) -> u32 {
    let mut checksum = 0u32;

    for (index, byte) in bytes.iter().copied().enumerate() {
        let mut value = if (12..16).contains(&index) {
            0
        } else {
            byte as u32
        };

        if checksum & 0x8000_0000 != 0 {
            value = value.wrapping_add(1);
        }

        checksum = checksum.wrapping_mul(2).wrapping_add(value);
    }

    checksum
}

pub fn fix_header(raw: &mut [u8]) {
    if raw.len() < 16 {
        return;
    }
    // Set file size
    let len = raw.len() as u32;
    raw[8..12].copy_from_slice(&len.to_le_bytes());

    // Clear checksum before calculation
    raw[12..16].fill(0);

    let checksum = calculate_checksum(raw);
    raw[12..16].copy_from_slice(&checksum.to_le_bytes());
}
