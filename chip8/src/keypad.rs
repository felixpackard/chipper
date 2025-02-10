use anyhow::{bail, Context};

pub struct Keypad {
    pub(crate) keys: [u8; 0x10],
}

impl Keypad {
    pub fn new() -> Self {
        Self { keys: [0; 0x10] }
    }

    pub fn keydown(&mut self, scancode: u32) -> anyhow::Result<()> {
        let idx = Keypad::scancode_to_index(scancode).context("parse scancode during keydown")?;
        self.keys[idx] = 1;
        Ok(())
    }

    pub fn keyup(&mut self, scancode: u32) -> anyhow::Result<()> {
        let idx = Keypad::scancode_to_index(scancode).context("parse scancode during keyup")?;
        self.keys[idx] = 0;
        Ok(())
    }

    pub fn is_key_down(&self, key: u8) -> bool {
        self.keys[key as usize] == 1
    }

    pub fn is_key_up(&self, key: u8) -> bool {
        self.keys[key as usize] == 0
    }

    fn scancode_to_index(scancode: u32) -> anyhow::Result<usize> {
        Ok(match scancode {
            02 => 0x0, // 1 -> 1
            03 => 0x1, // 2 -> 2
            04 => 0x2, // 3 -> 3
            05 => 0xC, // 4 -> C
            16 => 0x3, // Q -> 4
            17 => 0x4, // W -> 5
            18 => 0x5, // E -> 6
            19 => 0xD, // R -> D
            30 => 0x6, // A -> 7
            31 => 0x7, // S -> 8
            32 => 0x8, // D -> 9
            33 => 0xE, // F -> E
            44 => 0xA, // Z -> A
            45 => 0x0, // X -> 0
            46 => 0xB, // C -> B
            47 => 0xF, // V -> F
            _ => bail!("encountered invalid scancode"),
        })
    }
}
