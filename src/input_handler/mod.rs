use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::current;

use crossterm::{
    event::{DisableMouseCapture, KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind},
    execute,
};
use parking_lot::Mutex;
use ratatui::layout::Rect;
use tokio::{
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};

mod message;

use crate::{
    app_data::{AppData, DockerControls, Header},
    app_error::AppError,
    docker_data::DockerMessage,
    ui::{DeleteButton, GuiState, SelectablePanel, Status, Ui, NavPanel},
};
pub use message::InputMessages;

/// Handle all input events
#[derive(Debug)]
pub struct InputHandler {
    app_data: Arc<Mutex<AppData>>,
    docker_sender: Sender<DockerMessage>,
    gui_state: Arc<Mutex<GuiState>>,
    info_sleep: Option<JoinHandle<()>>,
    is_running: Arc<AtomicBool>,
    mouse_capture: bool,
    rec: Receiver<InputMessages>,
}

impl InputHandler {
    /// Initialize self, and running the message handling loop
    pub async fn init(
        app_data: Arc<Mutex<AppData>>,
        rec: Receiver<InputMessages>,
        docker_sender: Sender<DockerMessage>,
        gui_state: Arc<Mutex<GuiState>>,
        is_running: Arc<AtomicBool>,
    ) {
        let mut inner = Self {
            app_data,
            docker_sender,
            gui_state,
            is_running,
            rec,
            mouse_capture: true,
            info_sleep: None,
        };
        inner.start().await;
    }

    /// check for incoming messages
    async fn start(&mut self) {
        while let Some(message) = self.rec.recv().await {
            match message {
                InputMessages::ButtonPress(key) => self.button_press(key.0, key.1).await,
                InputMessages::MouseEvent(mouse_event) => {
                    let error_or_help = self.gui_state.lock().status_contains(&[
                        Status::Error,
                        Status::Help,
                        Status::DeleteConfirm,
                    ]);
                    if !error_or_help {
                        self.mouse_press(mouse_event);
                    }
                    let delete_confirm = self
                        .gui_state
                        .lock()
                        .status_contains(&[Status::DeleteConfirm]);
                    if delete_confirm {
                        self.button_intersect(mouse_event).await;
                    }
                }
            }
            if !self.is_running.load(Ordering::SeqCst) {
                break;
            }
        }
    }

    /// Toggle the mouse capture (via input of the 'm' key)
    fn m_key(&mut self) {
        if self.mouse_capture {
            if execute!(std::io::stdout(), DisableMouseCapture).is_ok() {
                self.gui_state
                    .lock()
                    .set_info_box("✖ mouse capture disabled".to_owned());
            } else {
                self.app_data
                    .lock()
                    .set_error(AppError::MouseCapture(false));
                self.gui_state.lock().status_push(Status::Error);
            }
        } else if Ui::enable_mouse_capture().is_ok() {
            self.gui_state
                .lock()
                .set_info_box("✓ mouse capture enabled".to_owned());
        } else {
            self.app_data.lock().set_error(AppError::MouseCapture(true));
            self.gui_state.lock().status_push(Status::Error);
        };

        // If the info box sleep handle is currently being executed, as in 'm' is pressed twice within a 4000ms window
        // then cancel the first handle, as a new handle will be invoked
        if let Some(info_sleep_timer) = self.info_sleep.as_ref() {
            info_sleep_timer.abort();
        }

        let gui_state = Arc::clone(&self.gui_state);
        // Show the info box - with "mouse capture enabled / disabled", for 4000 ms
        self.info_sleep = Some(tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(4000)).await;
            gui_state.lock().reset_info_box();
        }));

        self.mouse_capture = !self.mouse_capture;
    }

    /// Sort the containers by a given header
    fn sort(&self, selected_header: Header) {
        self.app_data.lock().set_sort_by_header(selected_header);
    }

    /// Send a quit message to docker, to abort all spawns, if an error is returned, set is_running to false here instead
    /// If gui_status is Error or Init, then just set the is_running to false immediately, for a quicker exit
    async fn quit(&self) {
        let error_init = self
            .gui_state
            .lock()
            .status_contains(&[Status::Error, Status::Init]);
        if error_init || self.docker_sender.send(DockerMessage::Quit).await.is_err() {
            self.is_running
                .store(false, std::sync::atomic::Ordering::SeqCst);
        }
    }

    /// This is executed from the Delete Confirm dialog, and will send an internal message to actually remove the given container
    async fn confirm_delete(&self) {
        let id = self.gui_state.lock().get_delete_container();
        if let Some(id) = id {
            self.docker_sender
                .send(DockerMessage::Delete(id))
                .await
                .ok();
        }
    }

    /// This is executed from the Delete Confirm dialog, and will clear the delete_container information (removes id and closes panel)
    fn clear_delete(&self) {
        self.gui_state.lock().set_delete_container(None);
    }

    /// Handle any keyboard button events
    #[allow(clippy::too_many_lines)]
    async fn button_press(&mut self, key_code: KeyCode, key_modififer: KeyModifiers) {
        // TODO - refactor this to a single call, maybe return Error, Help or Normal
        let contains_error = self.gui_state.lock().status_contains(&[Status::Error]);
        let contains_help = self.gui_state.lock().status_contains(&[Status::Help]);
        let contains_delete = self
            .gui_state
            .lock()
            .status_contains(&[Status::DeleteConfirm]);

        // Always just quit on Ctrl + c/C or q/Q
        let is_c = || key_code == KeyCode::Char('c') || key_code == KeyCode::Char('C');
        let is_q = || key_code == KeyCode::Char('q') || key_code == KeyCode::Char('Q');
        if key_modififer == KeyModifiers::CONTROL && is_c() || is_q() {
            self.quit().await;
        }

        if contains_error {
            if let KeyCode::Char('c' | 'C') = key_code {
                self.app_data.lock().remove_error();
                self.gui_state.lock().status_del(Status::Error);
            }
        } else if contains_help {
            match key_code {
                KeyCode::Char('h' | 'H') => self.gui_state.lock().status_del(Status::Help),
                KeyCode::Char('m' | 'M') => self.m_key(),
                _ => (),
            }
        } else if contains_delete {
            match key_code {
                KeyCode::Char('y' | 'Y') => self.confirm_delete().await,
                KeyCode::Char('n' | 'N') => self.clear_delete(),
                _ => (),
            }
        } else {
            match key_code {
                KeyCode::Enter | KeyCode::Char('l') => {
                    let current_panel = self.gui_state.lock().get_current_nav().clone();
                    if (current_panel == NavPanel::Containers) {
                        self.gui_state.lock().append_nav(NavPanel::Logs {
                            container_name: self.app_data.lock().get_selected_container_name().unwrap_or_else(|| "N/A".into())
                        })
                    }
                },
                KeyCode::Esc => self.gui_state.lock().back_in_nav(),

                KeyCode::Char('0') => self.app_data.lock().reset_sorted(),
                KeyCode::Char('1') => self.sort(Header::State),
                KeyCode::Char('2') => self.sort(Header::Status),
                KeyCode::Char('3') => self.sort(Header::Cpu),
                KeyCode::Char('4') => self.sort(Header::Memory),
                KeyCode::Char('5') => self.sort(Header::Id),
                KeyCode::Char('6') => self.sort(Header::Name),
                KeyCode::Char('7') => self.sort(Header::Image),
                KeyCode::Char('8') => self.sort(Header::Rx),
                KeyCode::Char('9') => self.sort(Header::Tx),
                KeyCode::Char('h' | 'H') => self.gui_state.lock().status_push(Status::Help),
                KeyCode::Char('m' | 'M') => self.m_key(),

                KeyCode::Home => {
                    let mut locked_data = self.app_data.lock();
                    match self.gui_state.lock().get_current_nav() {
                        NavPanel::Containers => locked_data.containers_start(),
                        NavPanel::Logs { container_name: _ } => locked_data.log_start(),
                    }
                }
                KeyCode::End => {
                    let mut locked_data = self.app_data.lock();
                    match self.gui_state.lock().get_current_nav() {
                        NavPanel::Containers => locked_data.containers_end(),
                        NavPanel::Logs { container_name: _ } => locked_data.log_end(),
                    }
                }
                KeyCode::Up => self.previous(),
                KeyCode::PageUp => {
                    for _ in 0..=6 {
                        self.previous();
                    }
                }
                KeyCode::Down => self.next(),
                KeyCode::PageDown => {
                    for _ in 0..=6 {
                        self.next();
                    }
                }
                _ => (),
            }
        }
    }

    /// Check if a button press interacts with either the yes or no buttons in the delete container confirm window
    async fn button_intersect(&mut self, mouse_event: MouseEvent) {
        if mouse_event.kind == MouseEventKind::Down(MouseButton::Left) {
            let intersect = self.gui_state.lock().button_intersect(Rect::new(
                mouse_event.column,
                mouse_event.row,
                1,
                1,
            ));

            if let Some(button) = intersect {
                match button {
                    DeleteButton::Yes => self.confirm_delete().await,
                    DeleteButton::No => self.clear_delete(),
                }
            }
        }
    }

    /// Handle mouse button events
    fn mouse_press(&mut self, mouse_event: MouseEvent) {
        match mouse_event.kind {
            MouseEventKind::ScrollUp => self.previous(),
            MouseEventKind::ScrollDown => self.next(),
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some(header) = self.gui_state.lock().header_intersect(Rect::new(
                    mouse_event.column,
                    mouse_event.row,
                    1,
                    1,
                )) {
                    self.sort(header);
                }

                self.gui_state.lock().panel_intersect(Rect::new(
                    mouse_event.column,
                    mouse_event.row,
                    1,
                    1,
                ));
            }
            _ => (),
        }
    }

    /// Change state to next, depending which panel is currently in focus
    fn next(&mut self) {
        let mut locked_data = self.app_data.lock();
        match self.gui_state.lock().get_current_nav() {
            NavPanel::Containers => locked_data.containers_next(),
            NavPanel::Logs {container_name: _}=> locked_data.log_next(),
        };
    }

    /// Change state to previous, depending which panel is currently in focus
    fn previous(&mut self) {
        let mut locked_data = self.app_data.lock();
        match self.gui_state.lock().get_current_nav() {
            NavPanel::Containers => locked_data.containers_previous(),
            NavPanel::Logs {container_name:_}=> locked_data.log_previous(),
        }
    }
}
