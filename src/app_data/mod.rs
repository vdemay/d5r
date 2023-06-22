use crate::{app_error::AppError, parse_args::CliArgs};

pub mod container_data;
pub mod container_state;

/// Global app_state, stored in an Arc<Mutex>
#[derive(Debug, Clone)]
pub struct AppData {
    pub container_data: container_data::ContainerData,
    pub error: Option<AppError>,
    pub args: CliArgs,
}

impl AppData {
    /// Generate a default app_state
    pub fn default(args: CliArgs) -> Self {
        Self {
            args,
            container_data: container_data::ContainerData::new(args),
            error: None,
        }
    }

    /// Error related methods

    /// return single app_state error
    pub const fn get_error(&self) -> Option<AppError> {
        self.error
    }

    /// remove single app_state error
    pub fn remove_error(&mut self) {
        self.error = None;
    }

    /// insert single app_state error
    pub fn set_error(&mut self, error: AppError) {
        self.error = Some(error);
    }

}
