use ratatui::widgets::ListState;

use crate::api::IssueRef;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Focus {
    MyWork,
    Recent,
    SavedViews,
    View,
    Stub(usize),
    Detail(DetailFocus),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetailFocus {
    pub issue: IssueRef,
    pub from: LeftPanel,
    pub view: DetailView,
}

impl DetailFocus {
    pub fn reading(issue: impl Into<IssueRef>, from: LeftPanel) -> Self {
        Self {
            issue: issue.into(),
            from,
            view: DetailView::Reading,
        }
    }

    pub fn with_view(&self, view: DetailView) -> Self {
        Self {
            view,
            ..self.clone()
        }
    }
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
    SavedViews,
    Stub(usize),
}

impl LeftPanel {
    pub fn focus(self) -> Focus {
        match self {
            LeftPanel::MyWork => Focus::MyWork,
            LeftPanel::Recent => Focus::Recent,
            LeftPanel::SavedViews => Focus::SavedViews,
            LeftPanel::Stub(index) => Focus::Stub(index),
        }
    }
}

impl Focus {
    pub fn left(&self) -> LeftPanel {
        match self {
            Focus::MyWork => LeftPanel::MyWork,
            Focus::Recent => LeftPanel::Recent,
            Focus::SavedViews | Focus::View => LeftPanel::SavedViews,
            Focus::Stub(index) => LeftPanel::Stub(*index),
            Focus::Detail(detail) => detail.from,
        }
    }

    pub fn detail(&self) -> Option<&DetailFocus> {
        match self {
            Focus::Detail(detail) => Some(detail),
            _ => None,
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
    NewestComment,
    Keep,
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
