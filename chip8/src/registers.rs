use std::ops::{Index, IndexMut};

const REGISTER_COUNT: usize = 0x10;

pub struct Register(u8);
pub struct Registers([u8; REGISTER_COUNT]);

impl Index<Register> for Registers {
    type Output = u8;

    fn index(&self, register: Register) -> &Self::Output {
        &self.0[register.0 as usize]
    }
}

impl IndexMut<Register> for Registers {
    fn index_mut(&mut self, register: Register) -> &mut Self::Output {
        &mut self.0[register.0 as usize]
    }
}

impl Registers {
    pub fn new() -> Self {
        Self([0; REGISTER_COUNT])
    }
}
