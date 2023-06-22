use std::borrow::Cow;

use crossterm::event::KeyCode;

#[derive(Debug, Default, Clone, Eq, Hash, PartialEq)]
pub enum NavPanel {
    #[default]
    Containers,
    Logs,
    Metrics,
}

pub enum Action {
    NavAction(String, KeyCode, NavPanel),
    BackAction(String, KeyCode),
    RunAction(String, KeyCode),
}

impl Action {
    pub fn label(&self) -> &str {
        match self {
            Self::NavAction(label, _, _) => label,
            Self::BackAction(label, _) => label,
            Self::RunAction(label, _) => label,
        }
    }

    pub fn key(&self) -> KeyCode {
        match self {
            Self::NavAction(_, k, _) => *k,
            Self::BackAction(_, k) => *k,
            Self::RunAction(_, k) => *k,
        }
    }
}

impl NavPanel {
    pub fn title(&self) -> Cow<'static, str> {
        match self {
            Self::Containers => "Containers".into(),
            Self::Logs => "Logs".into(),
            Self::Metrics => "Metrics".into(),
        }
    }

    pub fn all_actions(&self) -> Vec<Action> {
        let mut out: Vec<Action> = vec![];
        out.append(&mut self.actions_0());
        out.append(&mut self.actions_1());
        out.append(&mut self.actions_2());
        return out;
    }

    pub fn actions_0(&self) -> Vec<Action> {
        match self {
            Self::Containers => {
                vec![
                    Action::NavAction(String::from("(l) Logs"), KeyCode::Char('l'), NavPanel::Logs),
                    Action::NavAction(
                        String::from("(m) Metrics"),
                        KeyCode::Char('m'),
                        NavPanel::Metrics,
                    ),
                ]
            }
            Self::Logs => {
                vec![Action::BackAction(String::from("(Esc) back"), KeyCode::Esc)]
            }
            Self::Metrics => {
                vec![Action::BackAction(String::from("(Esc) back"), KeyCode::Esc)]
            }
        }
    }

    pub fn actions_1(&self) -> Vec<Action> {
        match self {
            Self::Containers => {
                vec![
                    Action::RunAction(String::from("(s) Start"), KeyCode::Char('s')),
                    Action::RunAction(String::from("(S) Stop"), KeyCode::Char('m')),
                ]
            }
            Self::Logs => {
                vec![]
            }
            Self::Metrics => {
                vec![]
            }
        }
    }
    pub fn actions_2(&self) -> Vec<Action> {
        match self {
            Self::Containers => {
                vec![]
            }
            Self::Logs => {
                vec![]
            }
            Self::Metrics => {
                vec![]
            }
        }
    }
}
