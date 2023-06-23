use crate::app_data::container_state::ContainerId;

#[derive(Debug, Clone)]
pub enum DockerMessage {
    DeleteContainer(ContainerId),
    ConfirmDeleteContainer(ContainerId),
    PauseContainer(ContainerId),
    RestartContainer(ContainerId),
    StartContainer(ContainerId),
    StopContainer(ContainerId),
    UnpauseContainer(ContainerId),
    InfosContainer(ContainerId),
    ShellContainer(ContainerId),
    Quit,
    Update,
}
