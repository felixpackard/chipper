mod display;
mod memory;

use std::fmt::Display as FmtDisplay;
use std::path::PathBuf;

use anyhow::Context;

use crate::display::Display;
use crate::memory::Memory;

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
pub const REGISTER_COUNT: usize = 0x10;

pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;

struct Opcode {
    c: u8,
    x: u8,
    y: u8,
    n: u8,
    nn: u8,
    nnn: u16,
}

pub struct Chip8 {
    /// Heap-allocated RAM that stores font data, ROMs, and is fully writeable
    memory: Memory,
    /// A bit-packed frame buffer containing binary pixel states
    display: Display,
    /// A stack for 16-bit addresses, which is used to call subroutines/functions and return from them
    stack: [u16; STACK_SIZE],
    /// A pointer to the current stack address in use
    sp: u8,
    /// 16 8-bit general-purpose variable registers numbered 0 through F hexadecimal
    v: [u8; REGISTER_COUNT],
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
            sp: 0,
            v: [0; REGISTER_COUNT],
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

    pub fn load_rom_from_file(&mut self, path: PathBuf) -> anyhow::Result<()> {
        let buf = std::fs::read(path).context("read rom file")?;
        self.load_rom(&buf).context("load rom from file")?;
        Ok(())
    }

    pub fn is_fb_dirty(&self) -> bool {
        self.display.dirty
    }

    pub fn fb(&mut self) -> crate::display::FrameBuffer {
        self.display.fb()
    }

    pub fn cycle(&mut self) {
        let opcode = self.fetch();
        let opcode = self.decode(opcode);
        self.execute(opcode);
    }

    fn fetch(&mut self) -> u16 {
        print!("{:#02x} ", self.pc);

        let pc = self.pc as usize;
        let b1 = self.memory.data[pc] as u16;
        let b2 = self.memory.data[pc + 1] as u16;

        self.pc += 2;

        b1 << 8 | b2
    }

    fn decode(&mut self, opcode: u16) -> Opcode {
        let c = ((opcode & 0xF000) >> 12) as u8;
        let x = ((opcode & 0x0F00) >> 8) as u8;
        let y = ((opcode & 0x00F0) >> 4) as u8;
        let n = (opcode & 0x000F) as u8;
        let nn = (opcode & 0x00FF) as u8;
        let nnn = opcode & 0x0FFF;

        Opcode {
            c,
            x,
            y,
            n,
            nn,
            nnn,
        }
    }

    fn execute(&mut self, opcode: Opcode) {
        match opcode.c {
            0x0 => match (opcode.x, opcode.y, opcode.n) {
                (0, 0xE, 0) => self.op_cls(),
                (0, 0xE, 0xE) => self.op_sub_return(),
                _ => todo!(),
            },
            0x1 => self.op_jump(opcode.nnn),
            0x2 => self.op_sub_call(opcode.nnn),
            0x6 => self.op_set(opcode.x, opcode.nn),
            0x7 => self.op_add(opcode.x, opcode.nn),
            0xA => self.op_set_index(opcode.nnn),
            0xD => self.op_display(opcode.x, opcode.y, opcode.n),
            _ => todo!(),
        }
    }

    /* Operations */

    /// 0x00E0
    fn op_cls(&mut self) {
        println!("op_cls(00E0)");
        self.display.clear();
    }

    /// 0x00EE
    fn op_sub_return(&mut self) {
        println!("op_sub_return(00EE)");
        assert!(self.sp > 0);
        self.sp -= 1;
        self.pc = self.stack[self.sp as usize];
    }

    /// 0x1NNN
    fn op_jump(&mut self, nnn: u16) {
        println!("op_jump(1NNN) {:#04x}", nnn);
        self.pc = nnn;
    }

    /// 0x2NNN
    fn op_sub_call(&mut self, nnn: u16) {
        println!("op_sub_call(2NNN)");
        self.stack[self.sp as usize] = self.pc;
        self.sp += 1;
        self.pc = nnn;
    }

    /// 0x6XNN
    fn op_set(&mut self, x: u8, nn: u8) {
        println!("op_set(6XNN) {:#02x} {:#02x}", x, nn);
        self.v[x as usize] = nn;
    }

    /// 0x7XNN
    fn op_add(&mut self, x: u8, nn: u8) {
        println!("op_add(7XNN) {:#02x} {:#02x}", x, nn);
        self.v[x as usize] = self.v[x as usize].saturating_add(nn);
    }

    /// 0xANNN
    fn op_set_index(&mut self, nnn: u16) {
        println!("op_set_index(0xANNN) {:#04x}", nnn);
        self.i = nnn;
    }

    /// 0xDXYN
    fn op_display(&mut self, x: u8, y: u8, n: u8) {
        println!("op_display(DXYN) {:#02x} {:#02x} {:#02x}", x, y, n);
        let vx = self.v[x as usize] as usize % SCREEN_WIDTH;
        let vy = self.v[y as usize] as usize % SCREEN_HEIGHT;
        self.v[0xF] = 0;

        'outer: for row in 0..n {
            let y = vy + row as usize;
            if y >= SCREEN_HEIGHT {
                break;
            }

            let byte = self.memory.data[self.i as usize + row as usize];
            for col in 0..8 {
                let x = vx + col;
                if x >= SCREEN_WIDTH {
                    break 'outer;
                }

                if (byte >> (7 - col)) & 0x1 == 1 {
                    println!("toggling {x}, {y}");
                    if self.display.toggle(x, y) {
                        self.v[0xF] = 1;
                    }
                }
            }
        }
    }
}

impl FmtDisplay for Chip8 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "=== Memory ===\n{}", self.memory)
    }
}

#[cfg(test)]
mod tests {
    use super::{Chip8, SCREEN_HEIGHT, SCREEN_WIDTH};

    #[test]
    fn test_op_cls() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0x00, 0xE0]).unwrap();
        assert_eq!(chip8.display.toggle(0, 0), false);
        chip8.cycle();
        assert_eq!(chip8.display.is_set(0, 0), false);
    }

    #[test]
    fn test_op_sub_return() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0x00, 0x00, 0x00, 0xEE]).unwrap();
        chip8.pc += 2;
        chip8.stack[0] = 0x200;
        chip8.sp += 1;
        chip8.cycle();
        assert_eq!(chip8.pc, 0x200);
    }

    #[test]
    fn test_op_jump() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0x11, 0x2C]).unwrap();
        chip8.cycle();
        assert_eq!(chip8.pc, 300);
    }

    #[test]
    fn test_op_sub_call() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0x00, 0x00, 0x22, 0x00]).unwrap();
        chip8.pc += 2;
        chip8.cycle();
        assert_eq!(chip8.pc, 0x200);
        assert_eq!(chip8.stack[0], 0x204);
        assert_eq!(chip8.sp, 1);
    }

    #[test]
    fn test_op_set() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0x60, 0xAA]).unwrap();
        chip8.cycle();
        assert_eq!(chip8.v[0], 0xAA);
    }

    #[test]
    fn test_op_add() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0x70, 0x10]).unwrap();
        chip8.v[0] = 16;
        chip8.cycle();
        assert_eq!(chip8.v[0], 32);
    }

    #[test]
    fn test_op_set_index() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0xA2, 0x22]).unwrap();
        chip8.cycle();
        assert_eq!(chip8.i, 0x222);
    }

    #[test]
    fn test_op_display() {
        let mut chip8 = Chip8::new().unwrap();
        #[rustfmt::skip]
        chip8.load_rom(&[
            0xD0, 0x12, // display
            0b00000010, // sprite
            0b00000001,
        ]).unwrap();

        let sx = SCREEN_WIDTH - 8;
        let sy = SCREEN_HEIGHT - 2;

        chip8.v[0] = sx as u8;
        chip8.v[1] = sy as u8;
        chip8.i = 0x202;
        chip8.cycle();

        assert_eq!(chip8.display.is_set(sx + 6, sy), true);
        assert_eq!(chip8.display.is_set(sx + 7, sy), false);
        assert_eq!(chip8.display.is_set(sx + 6, sy + 1), false);
        assert_eq!(chip8.display.is_set(sx + 7, sy + 1), true);
    }
}
