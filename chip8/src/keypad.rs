use anyhow::Context;

pub struct Keypad {
    pub(crate) keys: [u8; 0x10],
    pub(crate) awaiting_release: Option<u8>,
}

impl Keypad {
    pub fn new() -> Self {
        Self {
            keys: [0; 0x10],
            awaiting_release: None,
        }
    }

    pub fn keydown(&mut self, scancode: u32) -> anyhow::Result<()> {
        if let Some(key) =
            Keypad::scancode_to_index(scancode).context("parse scancode during keydown")?
        {
            self.keys[key] = 1;
        }
        Ok(())
    }

    pub fn keyup(&mut self, scancode: u32) -> anyhow::Result<()> {
        if let Some(key) =
            Keypad::scancode_to_index(scancode).context("parse scancode during keyup")?
        {
            self.keys[key] = 0;
        }
        Ok(())
    }

    pub fn await_release(&mut self, key: u8) {
        self.awaiting_release = Some(key);
    }

    pub fn process_release(&mut self) {
        self.awaiting_release = None;
    }

    pub fn is_key_down(&self, key: u8) -> bool {
        self.keys[key as usize] == 1
    }

    pub fn is_key_up(&self, key: u8) -> bool {
        self.keys[key as usize] == 0
    }

    fn scancode_to_index(scancode: u32) -> anyhow::Result<Option<usize>> {
        Ok(match scancode {
            18 => Some(0x1), // 1 -> 1
            19 => Some(0x2), // 2 -> 2
            20 => Some(0x3), // 3 -> 3
            21 => Some(0xC), // 4 -> C
            12 => Some(0x4), // Q -> 4
            13 => Some(0x5), // W -> 5
            14 => Some(0x6), // E -> 6
            15 => Some(0xD), // R -> D
            00 => Some(0x7), // A -> 7
            01 => Some(0x8), // S -> 8
            02 => Some(0x9), // D -> 9
            03 => Some(0xE), // F -> E
            06 => Some(0xA), // Z -> A
            07 => Some(0x0), // X -> 0
            08 => Some(0xB), // C -> B
            09 => Some(0xF), // V -> F
            _ => None,
        })
    }
}
