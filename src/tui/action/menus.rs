use super::keys::Action;

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
    Action::Comment,
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

pub const SAVED_VIEWS_MENU: &[Action] = &[
    Action::SelectNext,
    Action::Descend,
    Action::OpenInBrowser,
    Action::YankUrl,
    Action::Find,
    Action::Reload,
    Action::Ascend,
];

pub const VIEW_MENU: &[Action] = &[
    Action::SelectNext,
    Action::Descend,
    Action::OpenInBrowser,
    Action::YankUrl,
    Action::Ascend,
];

pub const STUB_MENU: &[Action] = &[Action::SelectNext, Action::Find, Action::Ascend];

pub const GLOBAL_MENU: &[Action] = &[
    Action::GoPrefix,
    Action::NextPanel,
    Action::PrevPanel,
    Action::Help,
    Action::Quit,
];
