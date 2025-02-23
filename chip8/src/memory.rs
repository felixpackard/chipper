use std::fmt::Display;

use anyhow::ensure;

pub struct Memory {
    pub(crate) data: [u8; crate::MEM_SIZE],
}

impl Memory {
    /// Create an empty instance of the Memory struct
    pub fn new() -> Self {
        Self {
            data: [0; crate::MEM_SIZE],
        }
    }

    /// Write `data` into memory starting at `addr` and return the number of bytes written
    pub fn write(&mut self, addr: usize, data: &[u8]) -> anyhow::Result<u16> {
        ensure!(addr < self.data.len(), "memory write address out of bounds");

        let available = self.data.len() - addr;
        ensure!(available >= data.len(), "memory write overflow");

        self.data[addr..addr + data.len()].copy_from_slice(&data[..data.len()]);
        Ok(available as u16)
    }
}

impl Display for Memory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const BYTES_PER_LINE: usize = 16;
        for (line, chunk) in self.data.chunks(BYTES_PER_LINE).enumerate() {
            write!(f, "{:04X}: ", line * BYTES_PER_LINE)?;
            for byte in chunk {
                write!(f, "{:02X} ", byte)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}
