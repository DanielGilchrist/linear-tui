use ratatui::widgets::ListState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    MyWork,
    Recent,
    Stub(usize),
    Detail(LeftPanel, DetailView),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailView {
    Reading,
    Comments,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeftPanel {
    MyWork,
    Recent,
    Stub(usize),
}

impl LeftPanel {
    pub fn focus(self) -> Focus {
        match self {
            LeftPanel::MyWork => Focus::MyWork,
            LeftPanel::Recent => Focus::Recent,
            LeftPanel::Stub(index) => Focus::Stub(index),
        }
    }
}

impl Focus {
    pub fn left(self) -> LeftPanel {
        match self {
            Focus::MyWork => LeftPanel::MyWork,
            Focus::Recent => LeftPanel::Recent,
            Focus::Stub(index) => LeftPanel::Stub(index),
            Focus::Detail(under, _) => under,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Prev,
    Next,
}

impl Direction {
    pub fn wrap(self, index: usize, len: usize) -> usize {
        match self {
            Direction::Next => (index + 1) % len,
            Direction::Prev => (index + len - 1) % len,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reveal {
    Top,
    Bottom,
}

pub enum Nav<'a> {
    List {
        state: &'a mut ListState,
        len: usize,
        viewport: usize,
    },
    Scroll {
        position: &'a mut usize,
        viewport: usize,
    },
}
