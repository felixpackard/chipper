mod display;
mod keypad;
mod memory;

use std::fmt::Display as FmtDisplay;
use std::path::PathBuf;

use anyhow::{bail, ensure, Context};
use rand::Rng;

use crate::display::Display;
use crate::keypad::Keypad;
use crate::memory::Memory;

pub const FONT_CHAR_LENGTH: usize = 5;

pub const FONT_DATA: [u8; FONT_CHAR_LENGTH * 0x10] = [
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

impl FmtDisplay for Opcode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{:#}{:#}", self.c, self.nnn)
    }
}

struct Chip8Config {
    legacy_shift: bool,
    jump_add_offset: bool,
    memory_increment_i: bool,
    print_operations: bool,
    ops_per_cycle: usize,
}

impl Chip8Config {
    pub fn new() -> Self {
        Self {
            legacy_shift: false,
            jump_add_offset: false,
            memory_increment_i: false,
            print_operations: false,
            ops_per_cycle: 11,
        }
    }
}

pub struct Chip8 {
    config: Chip8Config,
    /// Heap-allocated RAM that stores font data, ROMs, and is fully writeable
    memory: Memory,
    /// A frame buffer containing binary pixel states
    display: Display,
    /// A hexadecimal keypad containing 16 key states labelled 0 through F
    keypad: Keypad,
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
            config: Chip8Config::new(),
            memory,
            display: Display::new(),
            keypad: Keypad::new(),
            stack: [0; STACK_SIZE],
            sp: 0,
            v: [0; REGISTER_COUNT],
            pc: ROM_ADDR as u16,
            i: 0,
            dt: 0,
            st: 0,
        })
    }

    /* Config builder functions */
    pub fn legacy_shift(mut self, value: bool) -> Self {
        self.config.legacy_shift = value;
        self
    }

    pub fn jump_add_offset(mut self, value: bool) -> Self {
        self.config.jump_add_offset = value;
        self
    }

    pub fn memory_increment_i(mut self, value: bool) -> Self {
        self.config.memory_increment_i = value;
        self
    }

    pub fn print_operations(mut self, value: bool) -> Self {
        self.config.print_operations = value;
        self
    }

    pub fn ops_per_cycle(mut self, value: usize) -> Self {
        self.config.ops_per_cycle = value;
        self
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

    pub fn is_sound_playing(&self) -> bool {
        self.st > 0
    }

    pub fn fb(&mut self) -> crate::display::FrameBuffer {
        self.display.fb()
    }

    pub fn keydown(&mut self, scancode: u32) -> anyhow::Result<()> {
        self.keypad.keydown(scancode)
    }

    pub fn keyup(&mut self, scancode: u32) -> anyhow::Result<()> {
        self.keypad.keyup(scancode)
    }

    pub fn cycle(&mut self) {
        self.dt = self.dt.saturating_sub(1);
        self.st = self.st.saturating_sub(1);
        for _ in 0..self.config.ops_per_cycle {
            self.step();
        }
    }

    pub fn step(&mut self) {
        let opcode = self.fetch();
        let opcode = self.decode(opcode);
        self.execute(opcode);
    }

    fn fetch(&mut self) -> u16 {
        if self.config.print_operations {
            print!("{:#02x} ", self.pc);
        }

        let pc = self.pc as usize;
        let b1 = self.memory.data[pc] as u16;
        let b2 = self.memory.data[pc + 1] as u16;

        self.pc += 2;

        b1 << 8 | b2
    }

    fn decode(&mut self, opcode: u16) -> Opcode {
        Opcode {
            c: ((opcode & 0xF000) >> 12) as u8,
            x: ((opcode & 0x0F00) >> 8) as u8,
            y: ((opcode & 0x00F0) >> 4) as u8,
            n: (opcode & 0x000F) as u8,
            nn: (opcode & 0x00FF) as u8,
            nnn: opcode & 0x0FFF,
        }
    }

    fn execute(&mut self, opcode: Opcode) {
        match opcode.c {
            0x0 => match (opcode.x, opcode.y, opcode.n) {
                (0, 0xE, 0) => self.op_cls(),
                (0, 0xE, 0xE) => self.op_sub_return(),
                _ => self.bail_invalid_op(opcode).unwrap(),
            },
            0x1 => self.op_jump(opcode.nnn),
            0x2 => self.op_sub_call(opcode.nnn),
            0x3 => self.op_skip_eq(opcode.x, opcode.nn),
            0x4 => self.op_skip_ne(opcode.x, opcode.nn),
            0x5 => self.op_skip_reg_eq(opcode.x, opcode.y),
            0x6 => self.op_set(opcode.x, opcode.nn),
            0x7 => self.op_add(opcode.x, opcode.nn),
            0x8 => match opcode.n {
                0x0 => self.op_reg_set(opcode.x, opcode.y),
                0x1 => self.op_reg_or(opcode.x, opcode.y),
                0x2 => self.op_reg_and(opcode.x, opcode.y),
                0x3 => self.op_reg_xor(opcode.x, opcode.y),
                0x4 => self.op_reg_add(opcode.x, opcode.y),
                0x5 => self.op_reg_sub_right(opcode.x, opcode.y),
                0x6 => self.op_reg_shift_right(opcode.x, opcode.y),
                0x7 => self.op_reg_sub_left(opcode.x, opcode.y),
                0xE => self.op_reg_shift_left(opcode.x, opcode.y),
                _ => self.bail_invalid_op(opcode).unwrap(),
            },
            0x9 => self.op_skip_reg_ne(opcode.x, opcode.y),
            0xA => self.op_set_index(opcode.nnn),
            0xB => self.op_jump_with_offset(opcode.nnn, opcode.x),
            0xC => self.op_random(opcode.x, opcode.nn),
            0xD => self.op_display(opcode.x, opcode.y, opcode.n),
            0xE => match opcode.nn {
                0x9E => self.op_skip_if_key_down(opcode.x),
                0xA1 => self.op_skip_if_key_up(opcode.x),
                _ => self.bail_invalid_op(opcode).unwrap(),
            },
            0xF => match opcode.nn {
                0x07 => self.op_dt_get(opcode.x),
                0x0A => self.op_get_key(opcode.x),
                0x15 => self.op_dt_set(opcode.x),
                0x18 => self.op_st_set(opcode.x),
                0x1E => self.op_add_to_index(opcode.x),
                0x29 => self.op_font_character(opcode.x),
                0x33 => self.op_convert_to_decimal(opcode.x),
                0x55 => self.op_memory_store(opcode.x),
                0x65 => self.op_memory_load(opcode.x),
                _ => self.bail_invalid_op(opcode).unwrap(),
            },
            _ => self.bail_invalid_op(opcode).unwrap(),
        }
    }

    fn bail_invalid_op(&self, opcode: Opcode) -> anyhow::Result<()> {
        bail!(
            "invalid opcode '{}' encountered at {:#04x}",
            opcode,
            self.pc - 2
        );
    }

    fn print_op(&self, s: String) {
        if self.config.print_operations {
            println!("{}", s);
        }
    }

    /* Operations */

    /// 0x00E0
    fn op_cls(&mut self) {
        self.print_op(format!("op_cls(00E0)"));
        self.display.clear();
    }

    /// 0x00EE
    fn op_sub_return(&mut self) {
        self.print_op(format!("op_sub_return(00EE)"));
        assert!(self.sp > 0, "sub return with empty stack");
        self.sp -= 1;
        self.pc = self.stack[self.sp as usize];
    }

    /// 0x1NNN
    fn op_jump(&mut self, nnn: u16) {
        self.print_op(format!("op_jump(1NNN) {:#04x}", nnn));
        self.pc = nnn;
    }

    /// 0x2NNN
    fn op_sub_call(&mut self, nnn: u16) {
        self.print_op(format!("op_sub_call(2NNN) {:#04x}", nnn));
        assert!(self.sp < 12, "call stack overflow");
        self.stack[self.sp as usize] = self.pc;
        self.sp += 1;
        self.pc = nnn;
    }

    /// 0x3XNN
    fn op_skip_eq(&mut self, x: u8, nn: u8) {
        self.print_op(format!("op_jump_eq(3XNN) {:#02x} {:#02x}", x, nn));
        if self.v[x as usize] == nn {
            self.pc += 2;
        }
    }

    /// 0x4XNN
    fn op_skip_ne(&mut self, x: u8, nn: u8) {
        self.print_op(format!("op_skip_ne(0x4XNN) {:#02x} {:#02x}", x, nn));
        if self.v[x as usize] != nn {
            self.pc += 2;
        }
    }

    /// 0x5XY0
    fn op_skip_reg_eq(&mut self, x: u8, y: u8) {
        self.print_op(format!("op_skip_reg_eq(5XY0) {:#02x} {:#02x}", x, y));
        if self.v[x as usize] == self.v[y as usize] {
            self.pc += 2;
        }
    }

    /// 0x6XNN
    fn op_set(&mut self, x: u8, nn: u8) {
        self.print_op(format!("op_set(6XNN) {:#02x} {:#02x}", x, nn));
        self.v[x as usize] = nn;
    }

    /// 0x7XNN
    fn op_add(&mut self, x: u8, nn: u8) {
        self.print_op(format!("op_add(7XNN) {:#02x} {:#02x}", x, nn));
        self.v[x as usize] = self.v[x as usize].wrapping_add(nn);
    }

    /// 0x8XY0
    fn op_reg_set(&mut self, x: u8, y: u8) {
        self.print_op(format!("op_reg_set(8XY0) {:#02x} {:#02x}", x, y));
        self.v[x as usize] = self.v[y as usize];
    }

    /// 0x8XY1
    fn op_reg_or(&mut self, x: u8, y: u8) {
        self.print_op(format!("op_reg_or(8XY1) {:#02x} {:#02x}", x, y));
        self.v[x as usize] |= self.v[y as usize];
    }

    /// 0x8XY2
    fn op_reg_and(&mut self, x: u8, y: u8) {
        self.print_op(format!("op_reg_and(8XY2) {:#02x} {:#02x}", x, y));
        self.v[x as usize] &= self.v[y as usize];
    }

    /// 0x8XY3
    fn op_reg_xor(&mut self, x: u8, y: u8) {
        self.print_op(format!("op_reg_xor(8XY3) {:#02x} {:#02x}", x, y));
        self.v[x as usize] ^= self.v[y as usize];
    }

    /// 0x8XY4
    fn op_reg_add(&mut self, x: u8, y: u8) {
        self.print_op(format!("op_reg_add(8XY4) {:#02x} {:#02x}", x, y));
        let (sum, overflow) = self.v[x as usize].overflowing_add(self.v[y as usize]);
        self.v[x as usize] = sum;
        self.v[0xF] = overflow as u8;
    }

    /// 0x8XY5
    fn op_reg_sub_right(&mut self, x: u8, y: u8) {
        self.print_op(format!("op_reg_sub_right(8XY5) {:#02x} {:#02x}", x, y));
        let (sum, overflow) = self.v[x as usize].overflowing_sub(self.v[y as usize]);
        self.v[x as usize] = sum;
        self.v[0xF] = !overflow as u8;
    }

    /// 0x8XY6
    fn op_reg_shift_right(&mut self, x: u8, y: u8) {
        self.print_op(format!("op_reg_shift_right(8XY6) {:#02} {:#02}", x, y));
        if self.config.legacy_shift {
            self.v[x as usize] = self.v[y as usize];
        }
        let flag = self.v[x as usize] & 0x1;
        self.v[x as usize] >>= 1;
        self.v[0xF] = flag;
    }

    /// 0x8XY7
    fn op_reg_sub_left(&mut self, x: u8, y: u8) {
        self.print_op(format!("op_reg_sub_left(8XY7) {:#02x} {:#02x}", x, y));
        let (sum, overflow) = self.v[y as usize].overflowing_sub(self.v[x as usize]);
        self.v[x as usize] = sum;
        self.v[0xF] = !overflow as u8;
    }

    /// 0x8XYE
    fn op_reg_shift_left(&mut self, x: u8, y: u8) {
        self.print_op(format!("op_reg_shift_left(8XYE) {:#02} {:#02}", x, y));
        if self.config.legacy_shift {
            self.v[x as usize] = self.v[y as usize];
        }
        let flag = self.v[x as usize] >> 7 & 0x1;
        self.v[x as usize] <<= 1;
        self.v[0xF] = flag;
    }

    /// 0x9XY0
    fn op_skip_reg_ne(&mut self, x: u8, y: u8) {
        self.print_op(format!("op_skip_reg_ne(9XY0) {:#02x} {:#02x}", x, y));
        if self.v[x as usize] != self.v[y as usize] {
            self.pc += 2;
        }
    }

    /// 0xANNN
    fn op_set_index(&mut self, nnn: u16) {
        self.print_op(format!("op_set_index(ANNN) {:#04x}", nnn));
        self.i = nnn;
    }

    /// 0xBNNN
    fn op_jump_with_offset(&mut self, nnn: u16, x: u8) {
        self.print_op(format!("op_jump_with_offset(BNNN) {:#04x}", nnn));
        let idx = if self.config.jump_add_offset {
            x as usize
        } else {
            0
        };
        self.pc = nnn + self.v[idx] as u16;
    }

    /// 0xCNNN
    fn op_random(&mut self, x: u8, nn: u8) {
        self.print_op(format!("op_random(CXNN) {:#02x} {:#02x}", x, nn));
        self.v[x as usize] = nn & rand::rng().random::<u8>();
    }

    /// 0xDXYN
    fn op_display(&mut self, x: u8, y: u8, n: u8) {
        self.print_op(format!("op_display(DXYN) {:#02x} {:#02x} {:#02x}", x, y, n));
        let vx = self.v[x as usize] as usize % SCREEN_WIDTH;
        let vy = self.v[y as usize] as usize % SCREEN_HEIGHT;
        self.v[0xF] = 0;

        for row in 0..n as usize {
            let y = vy + row;
            if y >= SCREEN_HEIGHT {
                break;
            }

            let byte = self.memory.data[self.i as usize + row];
            for col in 0..8 {
                let x = vx + col;
                if x >= SCREEN_WIDTH {
                    break;
                }

                if (byte >> (7 - col)) & 0x1 == 1 {
                    if self.display.toggle(x, y) {
                        self.v[0xF] = 1;
                    }
                }
            }
        }
    }

    /// 0xEX9E
    fn op_skip_if_key_down(&mut self, x: u8) {
        self.print_op(format!("op_skip_if_key_down(EX9E) {:#02x}", x));
        if self.keypad.is_key_down(self.v[x as usize]) {
            self.pc += 2;
        }
    }

    /// 0xEXA1
    fn op_skip_if_key_up(&mut self, x: u8) {
        self.print_op(format!("op_skip_if_key_up(EXA1) {:#02x}", x));
        if self.keypad.is_key_up(self.v[x as usize]) {
            self.pc += 2;
        }
    }

    /// 0xFX07
    fn op_dt_get(&mut self, x: u8) {
        self.print_op(format!("op_dt_get(FX07) {:#02x}", x));
        self.v[x as usize] = self.dt;
    }

    /// 0xFX0A
    fn op_get_key(&mut self, x: u8) {
        self.print_op(format!("op_get_key(FX0A) {:#02x}", x));
        if let Some(key) = self.keypad.awaiting_release {
            if self.keypad.is_key_up(key) {
                self.v[x as usize] = key;
                self.keypad.process_release();
                return;
            }
        }
        for key in 0..0xF + 1 {
            if self.keypad.is_key_down(key) {
                self.keypad.await_release(key);
                break;
            }
        }
        self.pc -= 2;
    }

    /// 0xFX15
    fn op_dt_set(&mut self, x: u8) {
        self.print_op(format!("op_dt_set(FX15) {:#02x}", x));
        self.dt = self.v[x as usize];
    }

    /// 0xFX18
    fn op_st_set(&mut self, x: u8) {
        self.print_op(format!("op_st_set(FX18) {:#02x}", x));
        self.st = self.v[x as usize];
    }

    /// 0xFX1E
    fn op_add_to_index(&mut self, x: u8) {
        self.print_op(format!("op_add_to_index(FX1E) {:#02x}", x));
        self.i = self.i.wrapping_add(self.v[x as usize] as u16);
    }

    /// 0xFX29
    fn op_font_character(&mut self, x: u8) {
        self.print_op(format!("op_font_character(FX29) {:#02x}", x));
        self.i = (FONT_ADDR + FONT_CHAR_LENGTH * (self.v[x as usize] & 0xFF) as usize) as u16;
    }

    /// 0xFX33
    fn op_convert_to_decimal(&mut self, x: u8) {
        self.print_op(format!("op_convert_to_decimal(FX33) {:#02x}", x));
        let n = self.v[x as usize];
        assert!(self.i as usize + 2 < MEM_SIZE, "memory write overflow");
        self.memory.data[self.i as usize + 0] = n / 100 % 10;
        self.memory.data[self.i as usize + 1] = n / 10 % 10;
        self.memory.data[self.i as usize + 2] = n / 1 % 10;
    }

    /// 0xFX55
    fn op_memory_store(&mut self, x: u8) {
        self.print_op(format!("op_memory_store(FX55) {:#02x}", x));
        assert!(
            self.i as usize + (x as usize) < MEM_SIZE,
            "memory write overflow"
        );
        let start = self.i as usize;
        for i in 0..(x as usize) + 1 {
            self.memory.data[start + i] = self.v[i];
            if self.config.memory_increment_i {
                self.i += 1;
            }
        }
    }

    /// 0xFX65
    fn op_memory_load(&mut self, x: u8) {
        self.print_op(format!("op_memory_load(FX65) {:#02x}", x));
        assert!(
            self.i as usize + (x as usize) < MEM_SIZE,
            "memory read out of bounds"
        );
        let start = self.i as usize;
        for i in 0..(x as usize) + 1 {
            self.v[i] = self.memory.data[start + i];
            if self.config.memory_increment_i {
                self.i += 1;
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
    use super::{Chip8, FONT_CHAR_LENGTH, FONT_DATA, SCREEN_HEIGHT, SCREEN_WIDTH};

    #[test]
    fn test_op_cls() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0x00, 0xE0]).unwrap();
        assert_eq!(chip8.display.toggle(0, 0), false);
        chip8.step();
        assert_eq!(chip8.display.is_set(0, 0), false);
    }

    #[test]
    fn test_op_sub_return() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0x00, 0x00, 0x00, 0xEE]).unwrap();
        chip8.pc += 2;
        chip8.stack[0] = 0x200;
        chip8.sp += 1;
        chip8.step();
        assert_eq!(chip8.pc, 0x200);
    }

    #[test]
    fn test_op_jump() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0x11, 0x2C]).unwrap();
        chip8.step();
        assert_eq!(chip8.pc, 300);
    }

    #[test]
    fn test_op_sub_call() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0x00, 0x00, 0x22, 0x00]).unwrap();
        chip8.pc += 2;
        chip8.step();
        assert_eq!(chip8.pc, 0x200);
        assert_eq!(chip8.stack[0], 0x204);
        assert_eq!(chip8.sp, 1);
    }

    #[test]
    fn test_op_skip_eq() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0x30, 0x10]).unwrap();

        chip8.step();
        assert_eq!(chip8.pc, 0x202);

        chip8.pc = 0x200;
        chip8.v[0] = 0x10;
        chip8.step();
        assert_eq!(chip8.pc, 0x204);
    }

    #[test]
    fn test_op_skip_ne() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0x40, 0x10]).unwrap();

        chip8.step();
        assert_eq!(chip8.pc, 0x204);

        chip8.pc = 0x200;
        chip8.v[0] = 0x10;
        chip8.step();
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_op_skip_reg_eq() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0x50, 0x10]).unwrap();

        chip8.step();
        assert_eq!(chip8.pc, 0x204);

        chip8.pc = 0x200;
        chip8.v[1] = 0x10;
        chip8.step();
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_op_set() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0x60, 0xAA]).unwrap();
        chip8.step();
        assert_eq!(chip8.v[0], 0xAA);
    }

    #[test]
    fn test_op_add() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0x70, 0x20]).unwrap();

        chip8.v[0] = 16;
        chip8.step();
        assert_eq!(chip8.v[0], 48);

        chip8.pc = 0x200;
        chip8.v[0] = 0xFE;
        chip8.step();
        assert_eq!(chip8.v[0], 30);
    }

    #[test]
    fn test_op_reg_set() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0x80, 0x10]).unwrap();
        chip8.v[0] = 10;
        chip8.v[1] = 20;
        chip8.step();
        assert_eq!(chip8.v[0], 20);
    }

    #[test]
    fn test_op_reg_or() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0x80, 0x11]).unwrap();
        chip8.v[0] = 0b10010000;
        chip8.v[1] = 0b11000001;
        chip8.step();
        assert_eq!(chip8.v[0], 0b11010001);
    }

    #[test]
    fn test_op_reg_and() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0x80, 0x12]).unwrap();
        chip8.v[0] = 0b10010001;
        chip8.v[1] = 0b11000001;
        chip8.step();
        assert_eq!(chip8.v[0], 0b10000001);
    }

    #[test]
    fn test_op_reg_xor() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0x80, 0x13]).unwrap();
        chip8.v[0] = 0b10010001;
        chip8.v[1] = 0b11000001;
        chip8.step();
        assert_eq!(chip8.v[0], 0b01010000);
    }

    #[test]
    fn test_op_reg_add() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0x80, 0x14]).unwrap();

        chip8.v[0] = 200;
        chip8.v[1] = 100;
        chip8.step();
        assert_eq!(chip8.v[0], 44);
        assert_eq!(chip8.v[0xF], 1);

        chip8.pc = 0x200;
        chip8.v[0] = 10;
        chip8.v[1] = 20;
        chip8.step();
        assert_eq!(chip8.v[0], 30);
        assert_eq!(chip8.v[0xF], 0);
    }

    #[test]
    fn test_op_reg_sub_right() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0x80, 0x15]).unwrap();

        chip8.v[0] = 100;
        chip8.v[1] = 25;
        chip8.step();
        assert_eq!(chip8.v[0], 75);
        assert_eq!(chip8.v[0xF], 1);

        chip8.pc = 0x200;
        chip8.v[0] = 25;
        chip8.v[1] = 100;
        chip8.step();
        assert_eq!(chip8.v[0], 181);
        assert_eq!(chip8.v[0xF], 0);
    }

    #[test]
    fn test_op_reg_shift_right() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0x80, 0x16]).unwrap();

        chip8.v[0] = 0b00000100;
        chip8.step();
        assert_eq!(chip8.v[0], 0b00000010);
        assert_eq!(chip8.v[0xF], 0);

        chip8 = chip8.legacy_shift(true);
        chip8.pc = 0x200;
        chip8.v[0] = 0b0;
        chip8.v[1] = 0b00000101;
        chip8.step();
        assert_eq!(chip8.v[0], 0b00000010);
        assert_eq!(chip8.v[0xF], 1);
    }

    #[test]
    fn test_op_reg_sub_left() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0x80, 0x17]).unwrap();

        chip8.v[0] = 25;
        chip8.v[1] = 100;
        chip8.step();
        assert_eq!(chip8.v[0], 75);
        assert_eq!(chip8.v[0xF], 1);

        chip8.pc = 0x200;
        chip8.v[0] = 100;
        chip8.v[1] = 25;
        chip8.step();
        assert_eq!(chip8.v[0], u8::MIN);
        assert_eq!(chip8.v[0xF], 0);
    }

    #[test]
    fn test_op_reg_shift_left() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0x80, 0x1E]).unwrap();

        chip8.v[0] = 0b00100000;
        chip8.step();
        assert_eq!(chip8.v[0], 0b01000000);
        assert_eq!(chip8.v[0xF], 0);

        chip8 = chip8.legacy_shift(true);
        chip8.pc = 0x200;
        chip8.v[0] = 0b0;
        chip8.v[1] = 0b10100000;
        chip8.step();
        assert_eq!(chip8.v[0], 0b01000000);
        assert_eq!(chip8.v[0xF], 1);
    }

    #[test]
    fn test_op_skip_reg_ne() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0x90, 0x10]).unwrap();

        chip8.step();
        assert_eq!(chip8.pc, 0x202);

        chip8.pc = 0x200;
        chip8.v[1] = 0x10;
        chip8.step();
        assert_eq!(chip8.pc, 0x204);
    }

    #[test]
    fn test_op_set_index() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0xA2, 0x22]).unwrap();
        chip8.step();
        assert_eq!(chip8.i, 0x222);
    }

    #[test]
    fn test_op_jump_with_offset() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0xB3, 0x00]).unwrap();

        chip8.step();
        assert_eq!(chip8.pc, 0x300);

        chip8 = chip8.jump_add_offset(true);
        chip8.pc = 0x200;
        chip8.v[3] = 0x10;
        chip8.step();
        assert_eq!(chip8.pc, 0x300 + 0x10);
    }

    #[test]
    fn test_op_random() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0xC0, 0x10]).unwrap();
        chip8.step();
        // can't easily test random operation, so we just make sure the operation doesn't panic
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
        chip8.step();

        assert_eq!(chip8.display.is_set(sx + 6, sy), true);
        assert_eq!(chip8.display.is_set(sx + 7, sy), false);
        assert_eq!(chip8.display.is_set(sx + 6, sy + 1), false);
        assert_eq!(chip8.display.is_set(sx + 7, sy + 1), true);
    }

    #[test]
    fn test_op_skip_if_key_down() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0xE1, 0x9E]).unwrap();
        chip8.v[1] = 1;

        chip8.step();
        assert_eq!(chip8.pc, 0x202);

        chip8.pc = 0x200;
        chip8.keypad.keys[1] = 1;
        chip8.step();
        assert_eq!(chip8.pc, 0x204);
    }

    #[test]
    fn test_op_skip_if_key_up() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0xE1, 0xA1]).unwrap();
        chip8.v[1] = 1;

        chip8.step();
        assert_eq!(chip8.pc, 0x204);

        chip8.pc = 0x200;
        chip8.keypad.keys[1] = 1;
        chip8.step();
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_op_dt_get() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0xF0, 0x07]).unwrap();
        chip8.dt = 0x10;
        chip8.step();
        assert_eq!(chip8.v[0], 0x10);
    }

    #[test]
    fn test_op_get_key() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0xF0, 0x0A]).unwrap();

        chip8.step();
        assert_eq!(chip8.v[0], 0);
        assert_eq!(chip8.pc, 0x200);

        chip8.keypad.keys[0xF] = 1;
        chip8.step();
        chip8.step(); // shouldn't register key press until it's released
        assert_eq!(chip8.v[0], 0);
        assert_eq!(chip8.pc, 0x200);

        chip8.keypad.keys[0xF] = 0;
        chip8.step();
        assert_eq!(chip8.v[0], 0xF);
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_op_dt_set() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0xF0, 0x15]).unwrap();
        chip8.v[0] = 0x10;
        chip8.step();
        assert_eq!(chip8.dt, 0x10);
    }

    #[test]
    fn test_op_st_set() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0xF0, 0x18]).unwrap();
        chip8.v[0] = 0x10;
        chip8.step();
        assert_eq!(chip8.st, 0x10);
    }

    #[test]
    fn test_op_add_to_index() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0xF0, 0x1E]).unwrap();
        chip8.i = 0x10;
        chip8.v[0] = 0x10;
        chip8.step();
        assert_eq!(chip8.i, 0x20);
    }

    #[test]
    fn test_op_font_character() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0xF0, 0x29]).unwrap();

        chip8.v[0] = 0;
        chip8.step();
        assert_eq!(
            chip8.memory.data[chip8.i as usize..chip8.i as usize + FONT_CHAR_LENGTH],
            FONT_DATA[0..0 + FONT_CHAR_LENGTH]
        );

        chip8.pc = 0x200;
        chip8.v[0] = 0xF;
        chip8.step();
        assert_eq!(
            chip8.memory.data[chip8.i as usize..chip8.i as usize + FONT_CHAR_LENGTH],
            FONT_DATA[0xF * FONT_CHAR_LENGTH..0xF * FONT_CHAR_LENGTH + FONT_CHAR_LENGTH]
        );
    }

    #[test]
    fn test_op_convert_to_decimal() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0xF0, 0x33]).unwrap();
        chip8.v[0] = 156;
        chip8.i = 0x300;
        chip8.step();
        assert_eq!(
            chip8.memory.data[chip8.i as usize..chip8.i as usize + 3],
            [1, 5, 6]
        );
    }

    #[test]
    fn test_op_memory_store() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0xF3, 0x55]).unwrap();

        chip8.v[0..3].copy_from_slice(&[0x20, 0x10, 0x30]);
        chip8.i = 0x300;
        chip8.step();
        assert_eq!(chip8.i, 0x300);
        assert_eq!(chip8.memory.data[0x300..0x300 + 3], [0x20, 0x10, 0x30]);

        chip8 = chip8.memory_increment_i(true);
        chip8.pc = 0x200;
        chip8.step();
        assert_eq!(chip8.i, 0x300 + 4);
        assert_eq!(chip8.memory.data[0x300..0x300 + 3], [0x20, 0x10, 0x30]);
    }

    #[test]
    fn test_op_memory_load() {
        let mut chip8 = Chip8::new().unwrap();
        chip8.load_rom(&[0xF3, 0x65]).unwrap();

        chip8.memory.data[0x300..0x300 + 3].copy_from_slice(&[0x20, 0x10, 0x30]);
        chip8.i = 0x300;
        chip8.step();
        assert_eq!(chip8.i, 0x300);
        assert_eq!(chip8.v[0..3], [0x20, 0x10, 0x30]);

        chip8 = chip8.memory_increment_i(true);
        chip8.pc = 0x200;
        chip8.step();
        assert_eq!(chip8.i, 0x300 + 4);
        assert_eq!(chip8.v[0..3], [0x20, 0x10, 0x30]);
    }
}
