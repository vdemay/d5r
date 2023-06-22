use std::{borrow::Cow, sync::Arc};

use crate::{
    app_data::{container_state::State, AppData},
    docker_data::DockerMessage,
};
use crossterm::event::KeyCode;

use parking_lot::Mutex;

use super::GuiState;

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
    DockerMessageAction(String, KeyCode, DockerMessage),
}

impl Action {
    pub fn label(&self) -> &str {
        match self {
            Self::NavAction(label, _, _) => label,
            Self::BackAction(label, _) => label,
            Self::DockerMessageAction(label, _, _) => label,
        }
    }

    pub fn key(&self) -> KeyCode {
        match self {
            Self::NavAction(_, k, _) => *k,
            Self::BackAction(_, k) => *k,
            Self::DockerMessageAction(_, k, _) => *k,
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

    pub fn all_actions(
        &self,
        gui_state: &Arc<Mutex<GuiState>>,
        app_data: &Arc<Mutex<AppData>>,
    ) -> Vec<Action> {
        let mut out: Vec<Action> = vec![];
        out.append(&mut self.actions_0(gui_state, app_data));
        out.append(&mut self.actions_1(gui_state, app_data));
        out.append(&mut self.actions_2(gui_state, app_data));
        return out;
    }

    pub fn actions_0(
        &self,
        gui_state: &Arc<Mutex<GuiState>>,
        app_data: &Arc<Mutex<AppData>>,
    ) -> Vec<Action> {
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

    pub fn actions_1(
        &self,
        gui_state: &Arc<Mutex<GuiState>>,
        app_data: &Arc<Mutex<AppData>>,
    ) -> Vec<Action> {
        match self {
            Self::Containers => {
                let loading = gui_state.lock().is_loading();
                if loading {
                    vec![]
                } else {
                    let _app_data = app_data.lock();
                    let maybe_selected_container =
                        _app_data.container_data.get_selected_container();
                    if let Some(selected_container) = maybe_selected_container {
                        match selected_container.state {
                            State::Running => vec![
                                Action::DockerMessageAction(
                                    String::from("(r) Restart"),
                                    KeyCode::Char('r'),
                                    DockerMessage::Restart(selected_container.id.clone()),
                                ),
                                Action::DockerMessageAction(
                                    String::from("(S) Stop"),
                                    KeyCode::Char('S'),
                                    DockerMessage::Stop(selected_container.id.clone()),
                                ),
                                Action::DockerMessageAction(
                                    String::from("(p) Pause"),
                                    KeyCode::Char('p'),
                                    DockerMessage::Pause(selected_container.id.clone()),
                                ),
                            ],
                            State::Dead | State::Exited => vec![Action::DockerMessageAction(
                                String::from("(s) Start"),
                                KeyCode::Char('s'),
                                DockerMessage::Start(selected_container.id.clone()),
                            )],
                            State::Paused => vec![
                                Action::DockerMessageAction(
                                    String::from("(u) Unpause"),
                                    KeyCode::Char('u'),
                                    DockerMessage::Unpause(selected_container.id.clone()),
                                ),
                                Action::DockerMessageAction(
                                    String::from("(S) Stop"),
                                    KeyCode::Char('S'),
                                    DockerMessage::Stop(selected_container.id.clone()),
                                ),
                                Action::DockerMessageAction(
                                    String::from("(p) Pause"),
                                    KeyCode::Char('p'),
                                    DockerMessage::Pause(selected_container.id.clone()),
                                ),
                            ],
                            State::Restarting | State::Removing | State::Unknown => vec![],
                        }
                    } else {
                        vec![]
                    }
                }
            }
            Self::Logs => {
                vec![]
            }
            Self::Metrics => {
                vec![]
            }
        }
    }
    pub fn actions_2(
        &self,
        gui_state: &Arc<Mutex<GuiState>>,
        app_data: &Arc<Mutex<AppData>>,
    ) -> Vec<Action> {
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
