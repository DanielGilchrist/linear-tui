use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use KeyCode::{BackTab, Backspace, Char, Down, Enter, Esc, Left, PageDown, PageUp, Right, Tab, Up};

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

pub struct Binding<A: 'static> {
    pub action: A,
    pub keys: &'static [KeyCode],
    pub label: &'static str,
}

pub struct Keymap<A: 'static> {
    pub bindings: &'static [Binding<A>],
}

impl<A: Copy + PartialEq> Keymap<A> {
    pub fn resolve(&self, key: KeyEvent) -> Option<A> {
        self.bindings
            .iter()
            .find(|binding| binding.keys.contains(&key.code))
            .map(|binding| binding.action)
    }

    fn hint(&self, action: A) -> Option<String> {
        self.bindings
            .iter()
            .find(|binding| binding.action == action)
            .and_then(|binding| binding.keys.first())
            .map(|key| key_symbol(*key))
    }

    fn label(&self, action: A) -> Option<&'static str> {
        self.bindings
            .iter()
            .find(|binding| binding.action == action)
            .map(|binding| binding.label)
    }

    pub fn describe(&self, action: A) -> Option<(String, &'static str)> {
        self.bindings
            .iter()
            .find(|binding| binding.action == action)
            .map(|binding| {
                let keys = binding
                    .keys
                    .iter()
                    .map(|key| key_symbol(*key))
                    .collect::<Vec<_>>()
                    .join("/");

                (keys, binding.label)
            })
    }

    pub fn summary(&self) -> String {
        self.bindings
            .iter()
            .filter_map(|binding| {
                binding
                    .keys
                    .first()
                    .map(|key| format!("{} {}", key_symbol(*key), binding.label))
            })
            .collect::<Vec<_>>()
            .join("   ")
    }

    pub fn hint_bar(&self, specs: &[Hint<A>]) -> String {
        let parts: Vec<String> = specs
            .iter()
            .filter_map(|spec| match spec {
                Hint::Bound(action) => {
                    Some(format!("{} {}", self.hint(*action)?, self.label(*action)?))
                }
                Hint::Literal { keys, label } => Some(format!("{keys} {label}")),
            })
            .collect();
        parts.join("   ")
    }
}

pub enum Hint<A: 'static> {
    Bound(A),
    Literal {
        keys: &'static str,
        label: &'static str,
    },
}

fn key_symbol(code: KeyCode) -> String {
    match code {
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "enter".into(),
        KeyCode::Esc => "esc".into(),
        KeyCode::Tab => "tab".into(),
        KeyCode::BackTab => "shift+tab".into(),
        KeyCode::Left => "←".into(),
        KeyCode::Right => "→".into(),
        KeyCode::Up => "↑".into(),
        KeyCode::Down => "↓".into(),
        KeyCode::Backspace => "bksp".into(),
        other => format!("{other:?}"),
    }
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

pub const MY_WORK_HINTS: &[Hint<Action>] = &[
    Hint::Bound(Action::SelectNext),
    Hint::Bound(Action::NextView),
    Hint::Bound(Action::NextPanel),
    Hint::Literal {
        keys: "1-9",
        label: "jump",
    },
    Hint::Bound(Action::Descend),
    Hint::Bound(Action::OpenInBrowser),
    Hint::Bound(Action::YankUrl),
    Hint::Bound(Action::Find),
    Hint::Bound(Action::GoPrefix),
    Hint::Bound(Action::Quit),
];

pub const RECENT_HINTS: &[Hint<Action>] = &[
    Hint::Bound(Action::SelectNext),
    Hint::Bound(Action::NextPanel),
    Hint::Bound(Action::Descend),
    Hint::Bound(Action::OpenInBrowser),
    Hint::Bound(Action::YankUrl),
    Hint::Bound(Action::ClearRecent),
    Hint::Bound(Action::Find),
    Hint::Bound(Action::Quit),
];

pub const STUB_HINTS: &[Hint<Action>] = &[
    Hint::Bound(Action::SelectNext),
    Hint::Bound(Action::NextPanel),
    Hint::Literal {
        keys: "1-9",
        label: "jump",
    },
    Hint::Bound(Action::Ascend),
    Hint::Bound(Action::Quit),
];

pub const DETAIL_HINTS: &[Hint<Action>] = &[
    Hint::Literal {
        keys: "j/k",
        label: "scroll",
    },
    Hint::Literal {
        keys: "C-d/C-u",
        label: "page",
    },
    Hint::Bound(Action::SetStatus),
    Hint::Bound(Action::Assign),
    Hint::Bound(Action::OpenInBrowser),
    Hint::Bound(Action::YankUrl),
    Hint::Literal {
        keys: "tab/S-tab",
        label: "next/prev",
    },
    Hint::Bound(Action::Ascend),
    Hint::Bound(Action::Quit),
];

pub const PICKER_HINTS: &[Hint<PickerInput>] = &[
    Hint::Bound(PickerInput::Next),
    Hint::Bound(PickerInput::Accept),
    Hint::Bound(PickerInput::Cancel),
];

pub const CONFIRM_HINTS: &[Hint<ConfirmInput>] = &[
    Hint::Bound(ConfirmInput::Accept),
    Hint::Bound(ConfirmInput::Reject),
];

pub const MENU_HINTS: &[Hint<MenuInput>] = &[
    Hint::Bound(MenuInput::Next),
    Hint::Bound(MenuInput::SectionNext),
    Hint::Bound(MenuInput::Run),
    Hint::Bound(MenuInput::Close),
];

pub const INPUT_HINTS: &[Hint<InputInput>] = &[
    Hint::Literal {
        keys: "←/→",
        label: "move",
    },
    Hint::Bound(InputInput::Submit),
    Hint::Bound(InputInput::Cancel),
];

pub const MY_WORK_MENU: &[Action] = &[
    Action::SelectNext,
    Action::NextView,
    Action::Descend,
    Action::Find,
    Action::FindNext,
    Action::OpenInBrowser,
    Action::YankUrl,
    Action::Reload,
];

pub const DETAIL_MENU: &[Action] = &[
    Action::SetStatus,
    Action::Assign,
    Action::OpenInBrowser,
    Action::YankUrl,
    Action::Reload,
    Action::Ascend,
];

pub const RECENT_MENU: &[Action] = &[
    Action::SelectNext,
    Action::Descend,
    Action::Find,
    Action::OpenInBrowser,
    Action::YankUrl,
    Action::ClearRecent,
];

pub const STUB_MENU: &[Action] = &[Action::SelectNext, Action::Find, Action::Ascend];

pub const GLOBAL_MENU: &[Action] = &[
    Action::GoPrefix,
    Action::NextPanel,
    Action::PrevPanel,
    Action::Help,
    Action::Quit,
];
