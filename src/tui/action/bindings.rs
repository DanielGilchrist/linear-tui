use crossterm::event::KeyCode;
use KeyCode::{BackTab, Backspace, Char, Down, Enter, Esc, Left, PageDown, PageUp, Right, Tab, Up};

use super::keymap::{Binding, Keymap};
use super::keys::{Action, ConfirmInput, EditorInput, InputInput, MenuInput, PickerInput};

pub const BROWSE: Keymap<Action> = Keymap {
    bindings: &[
        Binding {
            action: Action::SelectNext,
            keys: &[Char('j'), Down],
            label: "move",
        },
        Binding {
            action: Action::SelectPrev,
            keys: &[Char('k'), Up],
            label: "move",
        },
        Binding {
            action: Action::NextView,
            keys: &[Char(']')],
            label: "view",
        },
        Binding {
            action: Action::PrevView,
            keys: &[Char('[')],
            label: "view",
        },
        Binding {
            action: Action::NextPanel,
            keys: &[Tab],
            label: "panel",
        },
        Binding {
            action: Action::PrevPanel,
            keys: &[BackTab],
            label: "panel",
        },
        Binding {
            action: Action::Descend,
            keys: &[Enter, Char('l'), Right],
            label: "open",
        },
        Binding {
            action: Action::Ascend,
            keys: &[Esc, Char('h'), Left, Backspace],
            label: "back",
        },
        Binding {
            action: Action::SetStatus,
            keys: &[Char('s')],
            label: "status",
        },
        Binding {
            action: Action::Assign,
            keys: &[Char('a')],
            label: "assign",
        },
        Binding {
            action: Action::Comment,
            keys: &[Char('c')],
            label: "comment",
        },
        Binding {
            action: Action::OpenInBrowser,
            keys: &[Char('o')],
            label: "browser",
        },
        Binding {
            action: Action::YankUrl,
            keys: &[Char('y')],
            label: "yank",
        },
        Binding {
            action: Action::Reload,
            keys: &[Char('r')],
            label: "reload",
        },
        Binding {
            action: Action::ClearRecent,
            keys: &[Char('x')],
            label: "clear",
        },
        Binding {
            action: Action::HalfPageDown,
            keys: &[PageDown],
            label: "page down",
        },
        Binding {
            action: Action::HalfPageUp,
            keys: &[PageUp],
            label: "page up",
        },
        Binding {
            action: Action::GoPrefix,
            keys: &[Char('g')],
            label: "go to",
        },
        Binding {
            action: Action::JumpToBottom,
            keys: &[Char('G')],
            label: "bottom",
        },
        Binding {
            action: Action::Find,
            keys: &[Char('/')],
            label: "find",
        },
        Binding {
            action: Action::FindNext,
            keys: &[Char('n')],
            label: "next match",
        },
        Binding {
            action: Action::FindPrev,
            keys: &[Char('N')],
            label: "prev match",
        },
        Binding {
            action: Action::Help,
            keys: &[Char('?')],
            label: "help",
        },
        Binding {
            action: Action::Quit,
            keys: &[Char('q')],
            label: "quit",
        },
    ],
};

pub const GO_GROUP: Keymap<Action> = Keymap {
    bindings: &[
        Binding {
            action: Action::JumpToTop,
            keys: &[Char('g')],
            label: "top",
        },
        Binding {
            action: Action::JumpToBottom,
            keys: &[Char('G')],
            label: "bottom",
        },
        Binding {
            action: Action::GoToIssue,
            keys: &[Char('i')],
            label: "issue",
        },
        Binding {
            action: Action::Search,
            keys: &[Char('s')],
            label: "search",
        },
    ],
};

pub const CTRL: Keymap<Action> = Keymap {
    bindings: &[
        Binding {
            action: Action::HalfPageDown,
            keys: &[Char('d')],
            label: "page down",
        },
        Binding {
            action: Action::HalfPageUp,
            keys: &[Char('u')],
            label: "page up",
        },
        Binding {
            action: Action::HistoryBack,
            keys: &[Char('o')],
            label: "prev issue",
        },
    ],
};

pub const DETAIL_KEYS: Keymap<Action> = Keymap {
    bindings: &[
        Binding {
            action: Action::HistoryForward,
            keys: &[Tab],
            label: "next issue",
        },
        Binding {
            action: Action::HistoryBack,
            keys: &[BackTab],
            label: "prev issue",
        },
    ],
};

pub const GO_MODAL: Keymap<Action> = Keymap {
    bindings: &[
        Binding {
            action: Action::JumpToTop,
            keys: &[Char('g')],
            label: "top",
        },
        Binding {
            action: Action::JumpToBottom,
            keys: &[Char('G')],
            label: "bottom",
        },
    ],
};

pub const PICKER: Keymap<PickerInput> = Keymap {
    bindings: &[
        Binding {
            action: PickerInput::Next,
            keys: &[Char('j'), Down],
            label: "move",
        },
        Binding {
            action: PickerInput::Prev,
            keys: &[Char('k'), Up],
            label: "move",
        },
        Binding {
            action: PickerInput::Accept,
            keys: &[Enter],
            label: "select",
        },
        Binding {
            action: PickerInput::Cancel,
            keys: &[Esc],
            label: "cancel",
        },
    ],
};

pub const CONFIRM: Keymap<ConfirmInput> = Keymap {
    bindings: &[
        Binding {
            action: ConfirmInput::Accept,
            keys: &[Enter, Char('y')],
            label: "confirm",
        },
        Binding {
            action: ConfirmInput::Reject,
            keys: &[Esc, Char('n')],
            label: "cancel",
        },
    ],
};

pub const MENU: Keymap<MenuInput> = Keymap {
    bindings: &[
        Binding {
            action: MenuInput::Next,
            keys: &[Char('j'), Down],
            label: "move",
        },
        Binding {
            action: MenuInput::Prev,
            keys: &[Char('k'), Up],
            label: "move",
        },
        Binding {
            action: MenuInput::SectionNext,
            keys: &[Tab],
            label: "section",
        },
        Binding {
            action: MenuInput::SectionPrev,
            keys: &[BackTab],
            label: "section",
        },
        Binding {
            action: MenuInput::Run,
            keys: &[Enter],
            label: "run",
        },
        Binding {
            action: MenuInput::Close,
            keys: &[Esc, Char('q'), Char('?')],
            label: "close",
        },
    ],
};

pub const INPUT: Keymap<InputInput> = Keymap {
    bindings: &[
        Binding {
            action: InputInput::MoveLeft,
            keys: &[Left],
            label: "move",
        },
        Binding {
            action: InputInput::MoveRight,
            keys: &[Right],
            label: "move",
        },
        Binding {
            action: InputInput::Submit,
            keys: &[Enter],
            label: "go",
        },
        Binding {
            action: InputInput::Cancel,
            keys: &[Esc],
            label: "cancel",
        },
        Binding {
            action: InputInput::Erase,
            keys: &[Backspace],
            label: "erase",
        },
    ],
};

pub const EDITOR: Keymap<EditorInput> = Keymap {
    bindings: &[
        Binding {
            action: EditorInput::Newline,
            keys: &[Enter],
            label: "newline",
        },
        Binding {
            action: EditorInput::Cancel,
            keys: &[Esc],
            label: "cancel",
        },
        Binding {
            action: EditorInput::Erase,
            keys: &[Backspace],
            label: "erase",
        },
        Binding {
            action: EditorInput::MoveLeft,
            keys: &[Left],
            label: "move",
        },
        Binding {
            action: EditorInput::MoveRight,
            keys: &[Right],
            label: "move",
        },
        Binding {
            action: EditorInput::MoveUp,
            keys: &[Up],
            label: "move",
        },
        Binding {
            action: EditorInput::MoveDown,
            keys: &[Down],
            label: "move",
        },
    ],
};
