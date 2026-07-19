use std::fmt;

const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

#[derive(Debug, Default, Clone, Copy)]
pub struct Spinner {
    frame: usize,
}

impl Spinner {
    pub fn tick(&mut self) {
        self.frame = self.frame.wrapping_add(1);
    }

    pub fn glyph(&self) -> &'static str {
        FRAMES[self.frame % FRAMES.len()]
    }
}

impl fmt::Display for Spinner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.glyph())
    }
}
