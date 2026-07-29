use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use KeyCode::Char;

use super::bindings::{
    BROWSE, CONFIRM, CTRL, EDITOR, INPUT, LABELS, MENU, PICKER, REACTIONS, WORKSPACES,
};

pub fn is_quit(key: &KeyEvent) -> bool {
    matches!(
        (key.modifiers, key.code),
        (KeyModifiers::CONTROL, KeyCode::Char('c'))
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    NextPanel,
    PrevPanel,
    Descend,
    Ascend,
    SelectNext,
    SelectPrev,
    NextView,
    PrevView,
    JumpToPanel(usize),
    Reload,
    OpenInBrowser,
    YankUrl,
    Edit,
    SetStatus,
    Assign,
    SetPriority,
    SetLabels,
    Comment,
    EnterComments,
    Reply,
    EditComment,
    DeleteComment,
    React,
    CycleGroup,
    CycleSort,
    ToggleZoom,
    ViewDisplay,
    ClearRecent,
    GoPrefix,
    GoToIssue,
    JumpToTop,
    JumpToBottom,
    Find,
    FindNext,
    FindPrev,
    Search,
    HalfPageDown,
    HalfPageUp,
    HistoryBack,
    HistoryForward,
    Workspaces,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerInput {
    Next,
    Prev,
    Accept,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmInput {
    Accept,
    Reject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuInput {
    Next,
    Prev,
    SectionNext,
    SectionPrev,
    Run,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputInput {
    MoveLeft,
    MoveRight,
    Submit,
    Cancel,
    Erase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReactionInput {
    Left,
    Right,
    Up,
    Down,
    Toggle,
    Custom,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelsInput {
    Next,
    Prev,
    Toggle,
    Submit,
    Cancel,
    Erase,
}

impl LabelsInput {
    pub fn from_key(key: KeyEvent) -> Option<LabelsInput> {
        LABELS.resolve(key)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorInput {
    Newline,
    Erase,
    Cancel,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
}

impl Action {
    pub fn from_key(key: KeyEvent) -> Option<Action> {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return CTRL.resolve(key);
        }

        if let Some(action) = BROWSE.resolve(key) {
            return Some(action);
        }

        if let KeyCode::Char(c @ '1'..='9') = key.code {
            return Some(Action::JumpToPanel(c as usize - '1' as usize));
        }

        None
    }
}

impl PickerInput {
    pub fn from_key(key: KeyEvent) -> Option<PickerInput> {
        PICKER.resolve(key)
    }
}

impl ConfirmInput {
    pub fn from_key(key: KeyEvent) -> Option<ConfirmInput> {
        CONFIRM.resolve(key)
    }
}

impl MenuInput {
    pub fn from_key(key: KeyEvent) -> Option<MenuInput> {
        MENU.resolve(key)
    }
}

impl InputInput {
    pub fn from_key(key: KeyEvent) -> Option<InputInput> {
        INPUT.resolve(key)
    }
}

impl EditorInput {
    pub fn from_key(key: KeyEvent) -> Option<EditorInput> {
        EDITOR.resolve(key)
    }
}

impl ReactionInput {
    pub fn from_key(key: KeyEvent) -> Option<ReactionInput> {
        REACTIONS.resolve(key)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspacesInput {
    Next,
    Prev,
    Accept,
    Cancel,
}

impl WorkspacesInput {
    pub fn from_key(key: KeyEvent) -> Option<WorkspacesInput> {
        WORKSPACES.resolve(key)
    }
}

pub fn is_editor_submit(key: KeyEvent) -> bool {
    key.code == Char('s') && key.modifiers.contains(KeyModifiers::CONTROL)
}
