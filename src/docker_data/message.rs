use crate::app_data::container_state::ContainerId;

#[derive(Debug, Clone)]
pub enum DockerMessage {
    Delete(ContainerId),
    ConfirmDelete(ContainerId),
    Pause(ContainerId),
    Quit,
    Restart(ContainerId),
    Start(ContainerId),
    Stop(ContainerId),
    Unpause(ContainerId),
    Infos(ContainerId),
    Update,
}
