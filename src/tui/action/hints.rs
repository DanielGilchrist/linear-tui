use super::keymap::Hint;
use super::keys::{Action, ConfirmInput, EditorInput, InputInput, MenuInput, PickerInput};

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
    Hint::Bound(Action::Comment),
    Hint::Literal {
        keys: "m",
        label: "comments",
    },
    Hint::Bound(Action::OpenInBrowser),
    Hint::Bound(Action::YankUrl),
    Hint::Literal {
        keys: "tab/S-tab",
        label: "next/prev",
    },
    Hint::Bound(Action::Ascend),
    Hint::Bound(Action::Quit),
];

pub const COMMENTS_HINTS: &[Hint<Action>] = &[
    Hint::Literal {
        keys: "j/k",
        label: "select",
    },
    Hint::Literal {
        keys: "r",
        label: "reply",
    },
    Hint::Literal {
        keys: "e",
        label: "edit",
    },
    Hint::Bound(Action::Comment),
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

pub const EDITOR_HINTS: &[Hint<EditorInput>] = &[
    Hint::Literal {
        keys: "C-s",
        label: "post",
    },
    Hint::Bound(EditorInput::Newline),
    Hint::Literal {
        keys: "↑/↓/←/→",
        label: "move",
    },
    Hint::Bound(EditorInput::Cancel),
];
