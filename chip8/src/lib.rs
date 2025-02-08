mod display;
mod memory;
mod registers;

use std::fmt::Display as FmtDisplay;

use anyhow::Context;

use crate::display::Display;
use crate::memory::Memory;
use crate::registers::Registers;

pub const FONT_DATA: [u8; 5 * 0x10] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

pub const FONT_ADDR: usize = 0x050;

pub const MEM_SIZE: usize = 0x1000;
pub const ROM_ADDR: usize = 0x200;
pub const STACK_SIZE: usize = 0x10;

pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;

pub struct Chip8 {
    /// Heap-allocated RAM that stores font data, ROMs, and is fully writeable
    memory: Memory,
    /// A bit-packed frame buffer containing binary pixel states
    display: Display,
    /// A stack for 16-bit addresses, which is used to call subroutines/functions and return from them
    stack: [u8; STACK_SIZE],
    /// A pointer to the current stack address in use
    stack_ptr: u8,
    /// 16 8-bit general-purpose variable registers numbered 0 through F hexadecimal
    v: Registers,
    /// The program counter points to the current instruction in memory
    pc: u16,
    /// The index register is used to point at locations in memory
    i: u16,
    /// The delay timer is decremented at a rate of 60 Hz until it reaches 0
    dt: u8,
    /// The sound timer is decremented at a rate of 60 Hz until it reaches 0, and plays a tone as long as it's not 0
    st: u8,
}

impl Chip8 {
    pub fn new() -> anyhow::Result<Self> {
        let mut memory = Memory::new();
        memory
            .write(FONT_ADDR, &FONT_DATA)
            .context("write font into memory")?;

        Ok(Chip8 {
            memory,
            display: Display::new(),
            stack: [0; STACK_SIZE],
            stack_ptr: 0,
            v: Registers::new(),
            pc: ROM_ADDR as u16,
            i: 0,
            dt: 0,
            st: 0,
        })
    }

    pub fn load_rom(&mut self, rom: &[u8]) -> anyhow::Result<()> {
        self.memory
            .write(ROM_ADDR, rom)
            .context("write rom into memory")?;
        self.pc = ROM_ADDR as u16;
        Ok(())
    }

    pub fn fb(&self) -> crate::display::FrameBuffer {
        self.display.fb()
    }
}

impl FmtDisplay for Chip8 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "=== Memory ===\n{}", self.memory)
    }
}
