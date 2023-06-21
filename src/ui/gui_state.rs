use ratatui::layout::{Constraint, Rect};
use std::{
    collections::{HashMap, HashSet},
    fmt,
};
use std::borrow::Cow;
use uuid::Uuid;

use crate::app_data::{ContainerId};


#[derive(Debug, Default, Clone, Eq, Hash, PartialEq)]
pub enum NavPanel {
    #[default]
    Containers,
    Logs ,
    Metrics
}

impl NavPanel {
    pub fn title(&self) -> Cow<'static, str> {
        match self {
            Self::Containers => "Containers".into(),
            Self::Logs =>"Logs".into(),
            Self::Metrics => "Metrics".into()
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum DeleteButton {
    Yes,
    No,
}

#[allow(unused)]
#[derive(Debug, Clone, Copy)]
pub enum BoxLocation {
    TopLeft,
    TopCentre,
    TopRight,
    MiddleLeft,
    MiddleCentre,
    MiddleRight,
    BottomLeft,
    BottomCentre,
    BottomRight,
}

impl BoxLocation {
    /// Screen is divided into 3x3 sections
    pub const fn get_indexes(self) -> (usize, usize) {
        match self {
            Self::TopLeft => (0, 0),
            Self::TopCentre => (0, 1),
            Self::TopRight => (0, 2),
            Self::MiddleLeft => (1, 0),
            Self::MiddleCentre => (1, 1),
            Self::MiddleRight => (1, 2),
            Self::BottomLeft => (2, 0),
            Self::BottomCentre => (2, 1),
            Self::BottomRight => (2, 2),
        }
    }

    /// Get both the vertical and hoziztonal constrains
    pub const fn get_constraints(
        self,
        blank_horizontal: u16,
        blank_vertical: u16,
        text_lines: u16,
        text_width: u16,
    ) -> ([Constraint; 3], [Constraint; 3]) {
        (
            Self::get_horizontal_constraints(self, blank_horizontal, text_width),
            Self::get_vertical_constraints(self, blank_vertical, text_lines),
        )
    }

    const fn get_horizontal_constraints(
        self,
        blank_horizontal: u16,
        text_width: u16,
    ) -> [Constraint; 3] {
        match self {
            Self::TopLeft | Self::MiddleLeft | Self::BottomLeft => [
                Constraint::Max(text_width),
                Constraint::Max(blank_horizontal),
                Constraint::Max(blank_horizontal),
            ],
            Self::TopCentre | Self::MiddleCentre | Self::BottomCentre => [
                Constraint::Max(blank_horizontal),
                Constraint::Max(text_width),
                Constraint::Max(blank_horizontal),
            ],
            Self::TopRight | Self::MiddleRight | Self::BottomRight => [
                Constraint::Max(blank_horizontal),
                Constraint::Max(blank_horizontal),
                Constraint::Max(text_width),
            ],
        }
    }

    const fn get_vertical_constraints(
        self,
        blank_vertical: u16,
        number_lines: u16,
    ) -> [Constraint; 3] {
        match self {
            Self::TopLeft | Self::TopCentre | Self::TopRight => [
                Constraint::Max(number_lines),
                Constraint::Max(blank_vertical),
                Constraint::Max(blank_vertical),
            ],
            Self::MiddleLeft | Self::MiddleCentre | Self::MiddleRight => [
                Constraint::Max(blank_vertical),
                Constraint::Max(number_lines),
                Constraint::Max(blank_vertical),
            ],
            Self::BottomLeft | Self::BottomCentre | Self::BottomRight => [
                Constraint::Max(blank_vertical),
                Constraint::Max(blank_vertical),
                Constraint::Max(number_lines),
            ],
        }
    }
}

/// State for the loading animation
#[derive(Debug, Default, Clone, Copy)]
pub enum Loading {
    #[default]
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
}

impl Loading {
    pub const fn next(self) -> Self {
        match self {
            Self::One => Self::Two,
            Self::Two => Self::Three,
            Self::Three => Self::Four,
            Self::Four => Self::Five,
            Self::Five => Self::Six,
            Self::Six => Self::Seven,
            Self::Seven => Self::Eight,
            Self::Eight => Self::Nine,
            Self::Nine => Self::Ten,
            Self::Ten => Self::One,
        }
    }
}

impl fmt::Display for Loading {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let disp = match self {
            Self::One => '⠋',
            Self::Two => '⠙',
            Self::Three => '⠹',
            Self::Four => '⠸',
            Self::Five => '⠼',
            Self::Six => '⠴',
            Self::Seven => '⠦',
            Self::Eight => '⠧',
            Self::Nine => '⠇',
            Self::Ten => '⠏',
        };
        write!(f, "{disp}")
    }
}

/// The application gui state can be in multiple of these four states at the same time
/// Various functions (e.g input handler), operate differently depending upon current Status
// Copy
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Status {
    Init,
    Help,
    DockerConnect,
    DeleteConfirm,
    Error,
}

/// Global gui_state, stored in an Arc<Mutex>
#[derive(Debug, Default, Clone)]
pub struct GuiState {
    is_loading: HashSet<Uuid>,
    loading_icon: Loading,
    delete_map: HashMap<DeleteButton, Rect>,
    status: HashSet<Status>,
    delete_container: Option<ContainerId>,
    pub info_box_text: Option<String>,
    pub nav: Vec<NavPanel>
}
impl GuiState {
    /// nav
    pub fn append_nav(&mut self, nav_panel : NavPanel){
        self.nav.insert(self.nav.len(), nav_panel)
    }

    pub fn back_in_nav(&mut self) {
        if self.nav.len() > 1 {
            self.nav.remove(self.nav.len() - 1);
        }
        return
    }

    pub fn get_current_nav(&mut self) -> &NavPanel {
        if self.nav.is_empty() {
            self.append_nav(NavPanel::Containers)
        }
        self.nav.last().unwrap()
    }


    /// Check if a given Rect (a clicked area of 1x1), interacts with any known delete button
    pub fn button_intersect(&mut self, rect: Rect) -> Option<DeleteButton> {
        self.delete_map
            .iter()
            .filter(|i| i.1.intersects(rect))
            .collect::<Vec<_>>()
            .get(0)
            .map(|data| *data.0)
    }


    /// Check if an ContainerId is set in the delete_container field
    pub fn get_delete_container(&self) -> Option<ContainerId> {
        self.delete_container.clone()
    }

    /// Set either a ContainerId, or None, to the delete_container field
    /// If Some, will also insert the DeleteConfirm status into self.status
    pub fn set_delete_container(&mut self, id: Option<ContainerId>) {
        if id.is_some() {
            self.status.insert(Status::DeleteConfirm);
        } else {
            self.delete_map.clear();
            self.status.remove(&Status::DeleteConfirm);
        }
        self.delete_container = id;
    }

    /// Check if the current gui_status contains any of the given status'
    /// Don't really like this methodology for gui state, needs a re-think
    pub fn status_contains(&self, status: &[Status]) -> bool {
        status.iter().any(|i| self.status.contains(i))
    }

    /// Remove a gui_status into the current gui_status HashSet
    pub fn status_del(&mut self, status: Status) {
        self.status.remove(&status);
        if status == Status::DeleteConfirm {
            self.status.remove(&Status::DeleteConfirm);
        }
    }

    /// Insert a gui_status into the current gui_status HashSet
    pub fn status_push(&mut self, status: Status) {
        self.status.insert(status);
    }


    /// Insert a new loading_uuid into HashSet, and advance the animation by one frame
    pub fn next_loading(&mut self, uuid: Uuid) {
        self.loading_icon = self.loading_icon.next();
        self.is_loading.insert(uuid);
    }

    /// If is_loading has any entries, return the current loading_icon, else an empty string, which needs to take up the same space, hence ' '
    pub fn get_loading(&mut self) -> String {
        if self.is_loading.is_empty() {
            String::from(" ")
        } else {
            self.loading_icon.to_string()
        }
    }

    /// Remove a loading_uuid from the is_loading HashSet
    pub fn remove_loading(&mut self, uuid: Uuid) {
        self.is_loading.remove(&uuid);
    }

    /// Set info box content
    pub fn set_info_box(&mut self, text: String) {
        self.info_box_text = Some(text);
    }

    /// Remove info box content
    pub fn reset_info_box(&mut self) {
        self.info_box_text = None;
    }
}
