pub struct Key(Option<usize>);

impl Key {
    pub fn from_scancode(value: u32) -> Self {
        match value {
            18 => Self(Some(0x1)), // 1 -> 1
            19 => Self(Some(0x2)), // 2 -> 2
            20 => Self(Some(0x3)), // 3 -> 3
            21 => Self(Some(0xC)), // 4 -> C
            12 => Self(Some(0x4)), // Q -> 4
            13 => Self(Some(0x5)), // W -> 5
            14 => Self(Some(0x6)), // E -> 6
            15 => Self(Some(0xD)), // R -> D
            00 => Self(Some(0x7)), // A -> 7
            01 => Self(Some(0x8)), // S -> 8
            02 => Self(Some(0x9)), // D -> 9
            03 => Self(Some(0xE)), // F -> E
            06 => Self(Some(0xA)), // Z -> A
            07 => Self(Some(0x0)), // X -> 0
            08 => Self(Some(0xB)), // C -> B
            09 => Self(Some(0xF)), // V -> F
            _ => Self(None),
        }
    }

    pub fn from_label(value: &str) -> Self {
        match value {
            "1" => Self(Some(0x1)), // 1 -> 1
            "2" => Self(Some(0x2)), // 2 -> 2
            "3" => Self(Some(0x3)), // 3 -> 3
            "4" => Self(Some(0xC)), // 4 -> C
            "q" => Self(Some(0x4)), // Q -> 4
            "w" => Self(Some(0x5)), // W -> 5
            "e" => Self(Some(0x6)), // E -> 6
            "r" => Self(Some(0xD)), // R -> D
            "a" => Self(Some(0x7)), // A -> 7
            "s" => Self(Some(0x8)), // S -> 8
            "d" => Self(Some(0x9)), // D -> 9
            "f" => Self(Some(0xE)), // F -> E
            "z" => Self(Some(0xA)), // Z -> A
            "x" => Self(Some(0x0)), // X -> 0
            "c" => Self(Some(0xB)), // C -> B
            "v" => Self(Some(0xF)), // V -> F
            _ => Self(None),
        }
    }
}

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

    pub fn keydown(&mut self, key: Key) -> anyhow::Result<()> {
        if let Some(key) = key.0 {
            self.keys[key] = 1;
        }
        Ok(())
    }

    pub fn keyup(&mut self, key: Key) -> anyhow::Result<()> {
        if let Some(key) = key.0 {
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
}
