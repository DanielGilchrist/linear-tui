use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use linear_tui::tui::action::{
    is_quit, Action, ConfirmInput, PickerInput, BROWSE, DETAIL_HINTS, MY_WORK_HINTS,
};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
}

#[test]
fn browse_keys_map_to_intents() {
    assert_eq!(Action::from_key(key(KeyCode::Char('j'))), Some(Action::SelectNext));
    assert_eq!(Action::from_key(key(KeyCode::Down)), Some(Action::SelectNext));
    assert_eq!(Action::from_key(key(KeyCode::Char('k'))), Some(Action::SelectPrev));
    assert_eq!(Action::from_key(key(KeyCode::Char('s'))), Some(Action::SetStatus));
    assert_eq!(Action::from_key(key(KeyCode::Char('o'))), Some(Action::OpenInBrowser));
    assert_eq!(Action::from_key(key(KeyCode::Enter)), Some(Action::Descend));
    assert_eq!(Action::from_key(key(KeyCode::Tab)), Some(Action::NextPanel));
    assert_eq!(Action::from_key(key(KeyCode::Esc)), Some(Action::Ascend));
    assert_eq!(Action::from_key(key(KeyCode::Char(']'))), Some(Action::NextView));
    assert_eq!(Action::from_key(key(KeyCode::Char('2'))), Some(Action::JumpToPanel(1)));
    assert_eq!(Action::from_key(key(KeyCode::Char('?'))), Some(Action::Help));
    assert_eq!(Action::from_key(key(KeyCode::Char('z'))), None);
}

#[test]
fn quit_is_ctrl_c_globally_and_q_while_browsing() {
    assert!(is_quit(&ctrl('c')));
    assert!(!is_quit(&key(KeyCode::Char('q'))));
    assert_eq!(Action::from_key(key(KeyCode::Char('q'))), Some(Action::Quit));
}

#[test]
fn modal_keymaps() {
    assert_eq!(PickerInput::from_key(key(KeyCode::Down)), Some(PickerInput::Next));
    assert_eq!(PickerInput::from_key(key(KeyCode::Enter)), Some(PickerInput::Accept));
    assert_eq!(PickerInput::from_key(key(KeyCode::Esc)), Some(PickerInput::Cancel));
    assert_eq!(PickerInput::from_key(key(KeyCode::Char('x'))), None);

    assert_eq!(
        ConfirmInput::from_key(key(KeyCode::Char('y'))),
        Some(ConfirmInput::Accept)
    );
    assert_eq!(
        ConfirmInput::from_key(key(KeyCode::Char('n'))),
        Some(ConfirmInput::Reject)
    );
    assert_eq!(ConfirmInput::from_key(key(KeyCode::Char('q'))), None);
}

#[test]
fn footer_is_derived_from_the_keymap() {
    let bar = BROWSE.hint_bar(MY_WORK_HINTS);
    assert!(bar.contains("enter open"), "got: {bar}");
    assert_eq!(Action::from_key(key(KeyCode::Enter)), Some(Action::Descend));

    assert!(bar.contains("o browser"));
    assert!(bar.contains("1-9 jump"));
    assert!(BROWSE.hint_bar(DETAIL_HINTS).contains("esc back"));
}
