use super::statefull_list::StatefulList;

/// Global app_state, stored in an Arc<Mutex>
#[derive(Debug, Clone)]
pub struct ImageData {
    containers: StatefulList<ContainerItem>,
    sorted_by: Option<(Header, SortedOrder)>,
    pub args: CliArgs,
}