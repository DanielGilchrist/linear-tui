use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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

use KeyCode::{Backspace, BackTab, Char, Down, Enter, Esc, Left, Right, Tab, Up};

pub const BROWSE: Keymap<Action> = Keymap {
    bindings: &[
        Binding { action: Action::SelectNext, keys: &[Char('j'), Down], label: "move" },
        Binding { action: Action::SelectPrev, keys: &[Char('k'), Up], label: "move" },
        Binding { action: Action::NextView, keys: &[Char(']')], label: "view" },
        Binding { action: Action::PrevView, keys: &[Char('[')], label: "view" },
        Binding { action: Action::NextPanel, keys: &[Tab], label: "panel" },
        Binding { action: Action::PrevPanel, keys: &[BackTab], label: "panel" },
        Binding { action: Action::Descend, keys: &[Enter, Char('l'), Right], label: "open" },
        Binding { action: Action::Ascend, keys: &[Esc, Char('h'), Left, Backspace], label: "back" },
        Binding { action: Action::SetStatus, keys: &[Char('s')], label: "status" },
        Binding { action: Action::Assign, keys: &[Char('a')], label: "assign" },
        Binding { action: Action::OpenInBrowser, keys: &[Char('o')], label: "browser" },
        Binding { action: Action::YankUrl, keys: &[Char('y')], label: "yank" },
        Binding { action: Action::Reload, keys: &[Char('r')], label: "reload" },
        Binding { action: Action::Quit, keys: &[Char('q')], label: "quit" },
    ],
};

pub const PICKER: Keymap<PickerInput> = Keymap {
    bindings: &[
        Binding { action: PickerInput::Next, keys: &[Char('j'), Down], label: "move" },
        Binding { action: PickerInput::Prev, keys: &[Char('k'), Up], label: "move" },
        Binding { action: PickerInput::Accept, keys: &[Enter], label: "select" },
        Binding { action: PickerInput::Cancel, keys: &[Esc], label: "cancel" },
    ],
};

pub const CONFIRM: Keymap<ConfirmInput> = Keymap {
    bindings: &[
        Binding { action: ConfirmInput::Accept, keys: &[Enter, Char('y')], label: "confirm" },
        Binding { action: ConfirmInput::Reject, keys: &[Esc, Char('n')], label: "cancel" },
    ],
};

pub const MY_WORK_HINTS: &[Hint<Action>] = &[
    Hint::Bound(Action::SelectNext),
    Hint::Bound(Action::NextView),
    Hint::Bound(Action::NextPanel),
    Hint::Literal { keys: "1-9", label: "jump" },
    Hint::Bound(Action::Descend),
    Hint::Bound(Action::OpenInBrowser),
    Hint::Bound(Action::YankUrl),
    Hint::Bound(Action::Quit),
];

pub const STUB_HINTS: &[Hint<Action>] = &[
    Hint::Bound(Action::SelectNext),
    Hint::Bound(Action::NextPanel),
    Hint::Literal { keys: "1-9", label: "jump" },
    Hint::Bound(Action::Ascend),
    Hint::Bound(Action::Quit),
];

pub const DETAIL_HINTS: &[Hint<Action>] = &[
    Hint::Literal { keys: "j/k", label: "scroll" },
    Hint::Bound(Action::SetStatus),
    Hint::Bound(Action::Assign),
    Hint::Bound(Action::OpenInBrowser),
    Hint::Bound(Action::YankUrl),
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
