const FB_SIZE: usize = crate::SCREEN_WIDTH * crate::SCREEN_HEIGHT / 8;

pub type FrameBuffer = [u8; FB_SIZE];

pub struct Display {
    /// A bit-packed representation on the frame buffer, where each bit corresponds with a single pixel that can be on (1) or off (0)
    fb: FrameBuffer,
}

impl Display {
    pub fn new() -> Self {
        Self { fb: [0; FB_SIZE] }
    }

    pub fn fb(&self) -> FrameBuffer {
        self.fb
    }
}
