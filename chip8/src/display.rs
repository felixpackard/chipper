use std::fmt::Display as FmtDisplay;

use crate::{SCREEN_HEIGHT, SCREEN_WIDTH};

// const FB_SIZE: usize = crate::SCREEN_WIDTH * crate::SCREEN_HEIGHT / 8;

pub type FrameBuffer = [[u8; SCREEN_WIDTH]; SCREEN_HEIGHT];

pub struct Display {
    /// A bit-packed representation on the frame buffer, where each bit corresponds with a single pixel that can be on (1) or off (0)
    pub(crate) fb: FrameBuffer,
    pub(crate) dirty: bool,
}

impl Display {
    pub fn new() -> Self {
        Self {
            fb: [[0; SCREEN_WIDTH]; SCREEN_HEIGHT],
            dirty: false,
        }
    }

    pub fn fb(&mut self) -> FrameBuffer {
        self.dirty = false;
        self.fb
    }

    /// Toggle the pixel at the coordinates and return true if it was already on
    /// This function marks the display as dirty, causing it to be re-rendered on the next update
    pub fn toggle(&mut self, x: usize, y: usize) -> bool {
        if x >= SCREEN_WIDTH || y >= SCREEN_HEIGHT {
            return false;
        }
        self.dirty = true;
        let prev = self.fb[y][x];
        self.fb[y][x] ^= 1;
        prev == 1
    }

    /// Clear the display contents by zeroing out the framebuffer
    /// This function marks the display as dirty, causing it to be re-rendered on the next update
    pub fn clear(&mut self) {
        self.dirty = true;
        for i in 0..SCREEN_HEIGHT {
            self.fb[i].fill(0);
        }
    }

    #[cfg(test)]
    pub fn is_set(&self, x: usize, y: usize) -> bool {
        self.fb[y][x] == 1
    }
}

impl FmtDisplay for Display {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                write!(f, "{}", self.fb[y][x])?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Display, SCREEN_HEIGHT, SCREEN_WIDTH};

    #[test]
    fn test_toggle() {
        let mut display = Display::new();
        assert_eq!(display.toggle(SCREEN_WIDTH - 1, SCREEN_HEIGHT - 1), false);
        assert_eq!(display.fb[SCREEN_HEIGHT - 1][SCREEN_WIDTH - 1], 1);
        assert_eq!(display.toggle(SCREEN_WIDTH - 1, SCREEN_HEIGHT - 1), true);
        assert_eq!(display.fb[SCREEN_HEIGHT - 1][SCREEN_WIDTH - 1], 0);
        assert_eq!(display.toggle(0, 0), false);
        assert_eq!(display.fb[0][0], 1);
        assert_eq!(display.toggle(0, 0), true);
        assert_eq!(display.fb[0][0], 0);
        assert_eq!(display.toggle(SCREEN_WIDTH, SCREEN_HEIGHT), false);
    }
}
