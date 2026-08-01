use std::num::NonZeroUsize;

use ratatui::widgets::ListState;

use super::overlay::Search;
use super::saved_views::ViewSurface;
use crate::api::{IssueRef, IssueSummary};

#[derive(Debug, Clone)]
pub enum Focus {
    MyWork,
    Recent,
    SavedViews,
    View(Box<ViewSurface>),
    Teams,
    Detail(DetailFocus),
}

#[derive(Debug, Clone)]
pub enum Origin {
    Panel(LeftPanel),
    View(Box<ViewSurface>),
    Search(Box<Search>),
}

impl Origin {
    pub fn panel(&self) -> LeftPanel {
        match self {
            Origin::Panel(panel) => *panel,
            Origin::View(_) => LeftPanel::SavedViews,
            Origin::Search(_) => LeftPanel::MyWork,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DetailFocus {
    pub issue: IssueRef,
    pub origin: Origin,
    pub view: DetailView,
    pub summary: Option<Box<IssueSummary>>,
}

impl DetailFocus {
    pub fn reading(issue: impl Into<IssueRef>, origin: Origin) -> Self {
        Self {
            issue: issue.into(),
            origin,
            view: DetailView::reading(),
            summary: None,
        }
    }

    pub fn with_view(&self, view: DetailView) -> Self {
        Self {
            view,
            ..self.clone()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scroll {
    #[default]
    Top,
    At(usize),
    Bottom,
}

impl Scroll {
    pub fn resolve(self, max: usize) -> usize {
        match self {
            Scroll::Top => 0,
            Scroll::At(row) => row.min(max),
            Scroll::Bottom => max,
        }
    }

    pub fn line(self) -> Option<usize> {
        match self {
            Scroll::Top => Some(0),
            Scroll::At(row) => Some(row),
            Scroll::Bottom => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor(usize);

impl Cursor {
    pub fn new(index: usize, len: usize) -> Option<Self> {
        (index < len).then_some(Cursor(index))
    }

    pub fn index(self) -> usize {
        self.0
    }

    pub fn clamped(self, len: usize) -> Cursor {
        Cursor(self.0.min(len.saturating_sub(1)))
    }

    pub fn stepped(self, len: usize, direction: Direction) -> Cursor {
        let Some(len) = NonZeroUsize::new(len) else {
            return self;
        };

        Cursor(direction.wrap(self.0, len))
    }

    pub fn edge(len: usize, edge: Edge) -> Cursor {
        match edge {
            Edge::Top => Cursor(0),
            Edge::Bottom => Cursor(len.saturating_sub(1)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailView {
    Reading { scroll: Scroll },
    Comments { at: Cursor },
}

impl DetailView {
    pub fn reading() -> Self {
        DetailView::Reading {
            scroll: Scroll::Top,
        }
    }

    pub fn comments(len: usize) -> Option<Self> {
        Cursor::new(0, len).map(|at| DetailView::Comments { at })
    }

    pub fn is_comments(&self) -> bool {
        matches!(self, DetailView::Comments { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeftPanel {
    MyWork,
    Recent,
    SavedViews,
    Teams,
}

pub const PANELS: [LeftPanel; 4] = [
    LeftPanel::MyWork,
    LeftPanel::Recent,
    LeftPanel::SavedViews,
    LeftPanel::Teams,
];

impl LeftPanel {
    pub fn focus(self) -> Focus {
        match self {
            LeftPanel::MyWork => Focus::MyWork,
            LeftPanel::Recent => Focus::Recent,
            LeftPanel::SavedViews => Focus::SavedViews,
            LeftPanel::Teams => Focus::Teams,
        }
    }
}

impl Focus {
    pub fn left(&self) -> LeftPanel {
        match self {
            Focus::MyWork => LeftPanel::MyWork,
            Focus::Recent => LeftPanel::Recent,
            Focus::SavedViews | Focus::View(_) => LeftPanel::SavedViews,
            Focus::Teams => LeftPanel::Teams,
            Focus::Detail(detail) => detail.origin.panel(),
        }
    }

    pub fn detail(&self) -> Option<&DetailFocus> {
        match self {
            Focus::Detail(detail) => Some(detail),
            _ => None,
        }
    }

    pub fn is_panel(&self, panel: LeftPanel) -> bool {
        match self {
            Focus::MyWork => panel == LeftPanel::MyWork,
            Focus::Recent => panel == LeftPanel::Recent,
            Focus::SavedViews => panel == LeftPanel::SavedViews,
            Focus::Teams => panel == LeftPanel::Teams,
            Focus::View(_) | Focus::Detail(_) => false,
        }
    }

    pub fn is_view(&self) -> bool {
        matches!(self, Focus::View(_))
    }

    pub fn open_view(&self) -> Option<&ViewSurface> {
        match self {
            Focus::View(surface) => Some(surface),
            Focus::Detail(detail) => match &detail.origin {
                Origin::View(surface) => Some(surface),
                Origin::Panel(_) | Origin::Search(_) => None,
            },
            Focus::MyWork | Focus::Recent | Focus::SavedViews | Focus::Teams => None,
        }
    }

    pub fn open_view_mut(&mut self) -> Option<&mut ViewSurface> {
        match self {
            Focus::View(surface) => Some(surface),
            Focus::Detail(detail) => match &mut detail.origin {
                Origin::View(surface) => Some(surface),
                Origin::Panel(_) | Origin::Search(_) => None,
            },
            Focus::MyWork | Focus::Recent | Focus::SavedViews | Focus::Teams => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Prev,
    Next,
}

impl Direction {
    pub fn wrap(self, index: usize, len: NonZeroUsize) -> usize {
        let len = len.get();

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
        scroll: &'a mut Scroll,
        viewport: usize,
        max: usize,
    },
    Comments {
        at: &'a mut Cursor,
        len: usize,
        viewport: usize,
    },
}

pub fn scrolled(scroll: Scroll, step: usize, direction: Direction, max: usize) -> Scroll {
    match direction {
        Direction::Next => match scroll {
            Scroll::Top => Scroll::At(step),
            Scroll::At(row) => Scroll::At(row + step),
            Scroll::Bottom => Scroll::Bottom,
        },
        Direction::Prev => match scroll {
            Scroll::Top => Scroll::Top,
            Scroll::At(row) if row <= step => Scroll::Top,
            Scroll::At(row) => Scroll::At(row - step),
            Scroll::Bottom => Scroll::At(max.saturating_sub(step)),
        },
    }
}

pub fn navigate_list(state: &mut ListState, len: usize, direction: Direction) {
    let Some(len) = NonZeroUsize::new(len) else {
        return;
    };

    let index = match state.selected() {
        Some(current) => direction.wrap(current, len),
        None => 0,
    };

    state.select(Some(index));
}

pub fn select_edge(state: &mut ListState, len: usize, edge: Edge) {
    if len == 0 {
        return;
    }

    state.select(Some(match edge {
        Edge::Bottom => len - 1,
        Edge::Top => 0,
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_view_cannot_open_on_an_empty_thread() {
        assert!(DetailView::comments(0).is_none());
        assert!(Cursor::new(3, 3).is_none());
        assert!(Cursor::new(2, 3).is_some());
    }
}
