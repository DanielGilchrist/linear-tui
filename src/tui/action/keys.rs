use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use KeyCode::Char;

use super::bindings::{BROWSE, CONFIRM, CTRL, EDITOR, INPUT, MENU, PICKER};

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
    SetStatus,
    Assign,
    Comment,
    EnterComments,
    Reply,
    EditComment,
    DeleteComment,
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

pub fn is_editor_submit(key: KeyEvent) -> bool {
    key.code == Char('s') && key.modifiers.contains(KeyModifiers::CONTROL)
}
