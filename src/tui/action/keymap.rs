use crossterm::event::{KeyCode, KeyEvent};

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
