use core::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use bollard::models::ContainerSummary;
use ratatui::widgets::{ListItem, ListState};

use crate::app_data::container_state::{
    ByteStats, Columns, ContainerId, ContainerItem, CpuStats, CpuTuple, LogsTz, MemTuple, State,
};
use crate::{parse_args::CliArgs, ui::log_sanitizer, ENTRY_POINT};

use super::statefull_list::StatefulList;

/// Global app_state, stored in an Arc<Mutex>
#[derive(Debug, Clone)]
pub struct ContainerData {
    containers: StatefulList<ContainerItem>,
    sorted_by: Option<(Header, SortedOrder)>,
    pub args: CliArgs,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SortedOrder {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum Header {
    State,
    Status,
    Cpu,
    Memory,
    Id,
    Name,
    Image,
    Rx,
    Tx,
}

/// Convert Header enum into strings to display
impl fmt::Display for Header {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let disp = match self {
            Self::State => "state",
            Self::Status => "status",
            Self::Cpu => "cpu",
            Self::Memory => "memory/limit",
            Self::Id => "id",
            Self::Name => "name",
            Self::Image => "image",
            Self::Rx => "↓ rx",
            Self::Tx => "↑ tx",
        };
        write!(f, "{disp:>x$}", x = f.width().unwrap_or(1))
    }
}

impl ContainerData {
    /// Generate a default container_state
    pub fn new(args: CliArgs) -> Self {
        Self {
            args,
            containers: StatefulList::new(vec![]),
            sorted_by: None,
        }
    }

    /// Change the sorted order, also set the selected container state to match new order
    fn set_sorted(&mut self, x: Option<(Header, SortedOrder)>) {
        self.sorted_by = x;
        self.sort_containers();
        self.containers
            .state
            .select(self.containers.items.iter().position(|i| {
                self.get_selected_container_id()
                    .map_or(false, |id| i.id == id)
            }));
    }

    /// Container sort related methods

    /// Remove the sorted header & order, and sort by default - created datetime
    pub fn reset_sorted(&mut self) {
        self.set_sorted(None);
    }

    /// Sort containers based on a given header, if headings match, and already ascending, remove sorting
    pub fn set_sort_by_header(&mut self, selected_header: Header) {
        let mut output = Some((selected_header, SortedOrder::Asc));
        if let Some((current_header, order)) = self.get_sorted() {
            if current_header == selected_header {
                match order {
                    SortedOrder::Desc => output = None,
                    SortedOrder::Asc => output = Some((selected_header, SortedOrder::Desc)),
                }
            }
        }
        self.set_sorted(output);
    }

    pub const fn get_sorted(&self) -> Option<(Header, SortedOrder)> {
        self.sorted_by
    }

    /// Sort the containers vec, based on a heading, either ascending or descending,
    /// If not sort set, then sort by created time
    pub fn sort_containers(&mut self) {
        if let Some((head, ord)) = self.sorted_by {
            match head {
                Header::State => match ord {
                    SortedOrder::Asc => self
                        .containers
                        .items
                        .sort_by(|a, b| b.state.order().cmp(&a.state.order())),
                    SortedOrder::Desc => self
                        .containers
                        .items
                        .sort_by(|a, b| a.state.order().cmp(&b.state.order())),
                },
                Header::Status => match ord {
                    SortedOrder::Asc => self
                        .containers
                        .items
                        .sort_by(|a, b| a.status.cmp(&b.status)),
                    SortedOrder::Desc => self
                        .containers
                        .items
                        .sort_by(|a, b| b.status.cmp(&a.status)),
                },
                Header::Cpu => match ord {
                    SortedOrder::Asc => self
                        .containers
                        .items
                        .sort_by(|a, b| a.cpu_stats.back().cmp(&b.cpu_stats.back())),
                    SortedOrder::Desc => self
                        .containers
                        .items
                        .sort_by(|a, b| b.cpu_stats.back().cmp(&a.cpu_stats.back())),
                },
                Header::Memory => match ord {
                    SortedOrder::Asc => self
                        .containers
                        .items
                        .sort_by(|a, b| a.mem_stats.back().cmp(&b.mem_stats.back())),
                    SortedOrder::Desc => self
                        .containers
                        .items
                        .sort_by(|a, b| b.mem_stats.back().cmp(&a.mem_stats.back())),
                },
                Header::Id => match ord {
                    SortedOrder::Asc => self.containers.items.sort_by(|a, b| a.id.cmp(&b.id)),
                    SortedOrder::Desc => self.containers.items.sort_by(|a, b| b.id.cmp(&a.id)),
                },
                Header::Image => match ord {
                    SortedOrder::Asc => self.containers.items.sort_by(|a, b| a.image.cmp(&b.image)),
                    SortedOrder::Desc => {
                        self.containers.items.sort_by(|a, b| b.image.cmp(&a.image));
                    }
                },
                Header::Name => match ord {
                    SortedOrder::Asc => self.containers.items.sort_by(|a, b| a.name.cmp(&b.name)),
                    SortedOrder::Desc => self.containers.items.sort_by(|a, b| b.name.cmp(&a.name)),
                },
                Header::Rx => match ord {
                    SortedOrder::Asc => self.containers.items.sort_by(|a, b| a.rx.cmp(&b.rx)),
                    SortedOrder::Desc => self.containers.items.sort_by(|a, b| b.rx.cmp(&a.rx)),
                },
                Header::Tx => match ord {
                    SortedOrder::Asc => self.containers.items.sort_by(|a, b| a.tx.cmp(&b.tx)),
                    SortedOrder::Desc => self.containers.items.sort_by(|a, b| b.tx.cmp(&a.tx)),
                },
            }
        } else {
            self.containers
                .items
                .sort_by(|a, b| a.created.cmp(&b.created));
        }
    }

    /// Container state methods

    /// Just get the total number of containers
    pub fn get_container_len(&self) -> usize {
        self.containers.items.len()
    }

    /// Get title for containers section
    pub fn container_title(&self) -> String {
        self.containers.get_state_title()
    }

    /// Select the first container
    pub fn containers_start(&mut self) {
        self.containers.start();
    }

    /// select the last container
    pub fn containers_end(&mut self) {
        self.containers.end();
    }

    /// Select the next container
    pub fn containers_next(&mut self) {
        self.containers.next();
    }

    /// select the previous container
    pub fn containers_previous(&mut self) {
        self.containers.previous();
    }

    /// Get Container items
    pub const fn get_container_items(&self) -> &Vec<ContainerItem> {
        &self.containers.items
    }

    /// Get Option of the current selected container
    pub fn get_selected_container(&self) -> Option<&ContainerItem> {
        self.containers
            .state
            .selected()
            .and_then(|i| self.containers.items.get(i))
    }

    /// Get mutable Option of the current selected container
    fn get_mut_selected_container(&mut self) -> Option<&mut ContainerItem> {
        self.containers
            .state
            .selected()
            .and_then(|i| self.containers.items.get_mut(i))
    }

    /// Get ListState of containers
    pub fn get_container_state(&mut self) -> &mut ListState {
        &mut self.containers.state
    }

    /// Logs related methods

    /// Get the title for log panel for selected container, will be either
    /// 1) "logs x/x - container_name" where container_name is 32 chars max
    /// 2) "logs - container_name" when no logs found, again 32 chars max
    /// 3) "" no container currently selected - aka no containers on system
    pub fn get_log_title(&self) -> String {
        self.get_selected_container().map_or_else(String::new, |y| {
            let logs_len = y.logs.get_state_title();
            let mut name = y.name.clone();
            name.truncate(32);
            if logs_len.is_empty() {
                format!("- {name} ")
            } else {
                format!("{logs_len} - {name}")
            }
        })
    }

    /// select next selected log line
    pub fn log_next(&mut self) {
        if let Some(i) = self.get_mut_selected_container() {
            i.logs.next();
        }
    }

    /// select previous selected log line
    pub fn log_previous(&mut self) {
        if let Some(i) = self.get_mut_selected_container() {
            i.logs.previous();
        }
    }

    /// select last selected log line
    pub fn log_end(&mut self) {
        if let Some(i) = self.get_mut_selected_container() {
            i.logs.end();
        }
    }

    /// select first selected log line
    pub fn info_start(&mut self) {
        if let Some(i) = self.get_mut_selected_container() {
            i.info.start();
        }
    }

    /// select next selected log line
    pub fn info_next(&mut self) {
        if let Some(i) = self.get_mut_selected_container() {
            i.info.next();
        }
    }

    /// select previous selected log line
    pub fn info_previous(&mut self) {
        if let Some(i) = self.get_mut_selected_container() {
            i.info.previous();
        }
    }

    /// select last selected log line
    pub fn info_end(&mut self) {
        if let Some(i) = self.get_mut_selected_container() {
            i.info.end();
        }
    }

    /// select first selected log line
    pub fn log_start(&mut self) {
        if let Some(i) = self.get_mut_selected_container() {
            i.logs.start();
        }
    }

    /// Chart data related methods

    /// Get mutable Option of the currently selected container chart data
    pub fn get_chart_data(&mut self) -> Option<(CpuTuple, MemTuple)> {
        self.containers
            .state
            .selected()
            .and_then(|i| self.containers.items.get_mut(i))
            .map(|i| i.get_chart_data())
    }

    /// Logs related methods

    /// Get mutable Vec of current containers logs
    pub fn get_logs(&mut self) -> Vec<ListItem<'static>> {
        self.containers
            .state
            .selected()
            .and_then(|i| self.containers.items.get_mut(i))
            .map_or(vec![], |i| i.logs.to_vec())
    }

    /// Get mutable Option of the currently selected container Logs state
    pub fn get_log_state(&mut self) -> Option<&mut ListState> {
        self.containers
            .state
            .selected()
            .and_then(|i| self.containers.items.get_mut(i))
            .map(|i| i.logs.state())
    }

    pub fn get_info_state(&mut self) -> Option<&mut ListState> {
        self.containers
            .state
            .selected()
            .and_then(|i| self.containers.items.get_mut(i))
            .map(|i| &mut i.info.state)
    }

    /// Check if the selected container is a dockerised version of oxker
    /// So that can disallow commands to be send
    /// Is a shabby way of implementing this
    pub fn is_oxker(&self) -> bool {
        self.get_selected_container().map_or(false, |i| i.is_oxker)
    }

    /// Check if the initial parsing has been completed, by making sure that all ids given (which are running) have a non empty cpu_stats vecdec
    pub fn initialised(&mut self, all_ids: &[(bool, ContainerId)]) -> bool {
        let count_is_running = all_ids.iter().filter(|i| i.0).count();
        let number_with_cpu_status = self
            .containers
            .items
            .iter()
            .filter(|i| !i.cpu_stats.is_empty())
            .count();
        count_is_running == number_with_cpu_status
    }

    /// Find the widths for the strings in the containers panel.
    /// So can display nicely and evenly
    pub fn get_width(&self) -> Columns {
        let mut columns = Columns::new();
        let count = |x: &String| u8::try_from(x.chars().count()).unwrap_or(12);

        // Should probably find a refactor here somewhere
        for container in &self.containers.items {
            let cpu_count = count(
                &container
                    .cpu_stats
                    .back()
                    .unwrap_or(&CpuStats::default())
                    .to_string(),
            );

            let mem_current_count = count(
                &container
                    .mem_stats
                    .back()
                    .unwrap_or(&ByteStats::default())
                    .to_string(),
            );

            columns.cpu.1 = columns.cpu.1.max(cpu_count);
            columns.image.1 = columns.image.1.max(count(&container.image));
            columns.mem.1 = columns.mem.1.max(mem_current_count);
            columns.mem.2 = columns.mem.2.max(count(&container.mem_limit.to_string()));
            columns.name.1 = columns.name.1.max(count(&container.name));
            columns.net_rx.1 = columns.net_rx.1.max(count(&container.rx.to_string()));
            columns.net_tx.1 = columns.net_tx.1.max(count(&container.tx.to_string()));
            columns.state.1 = columns.state.1.max(count(&container.state.to_string()));
            columns.status.1 = columns.status.1.max(count(&container.status));
        }
        columns
    }

    /// Update related methods

    /// return a mutable container by given id
    fn get_container_by_id(&mut self, id: &ContainerId) -> Option<&mut ContainerItem> {
        self.containers.items.iter_mut().find(|i| &i.id == id)
    }

    /// return a mutable container by given id
    pub fn get_container_name_by_id(&mut self, id: &ContainerId) -> Option<String> {
        self.containers
            .items
            .iter_mut()
            .find(|i| &i.id == id)
            .map(|i| i.name.clone())
    }

    /// Find the id of the currently selected container.
    /// If any containers on system, will always return a ContainerId
    /// Only returns None when no containers found.
    pub fn get_selected_container_id(&self) -> Option<ContainerId> {
        self.get_selected_container().map(|i| i.id.clone())
    }

    pub fn get_selected_container_name(&self) -> Option<String> {
        self.get_selected_container().map(|i| i.name.clone())
    }

    /// Update container mem, cpu, & network stats, in single function so only need to call .lock() once
    /// Will also, if a sort is set, sort the containers
    pub fn update_stats(
        &mut self,
        id: &ContainerId,
        cpu_stat: Option<f64>,
        mem_stat: Option<u64>,
        mem_limit: u64,
        rx: u64,
        tx: u64,
    ) {
        if let Some(container) = self.get_container_by_id(id) {
            if container.cpu_stats.len() >= 60 {
                container.cpu_stats.pop_front();
            }
            if container.mem_stats.len() >= 60 {
                container.mem_stats.pop_front();
            }

            if let Some(cpu) = cpu_stat {
                container.cpu_stats.push_back(CpuStats::new(cpu));
            }
            if let Some(mem) = mem_stat {
                container.mem_stats.push_back(ByteStats::new(mem));
            }

            container.rx.update(rx);
            container.tx.update(tx);
            container.mem_limit.update(mem_limit);
        }
        // need to benchmark this?
        self.sort_containers();
    }

    pub fn update_infos(&mut self, id: &ContainerId, info: &String) {
        if let Some(container) = self.get_container_by_id(id) {
            let mut out: Vec<ListItem<'_>> = vec![];
            info.lines()
                .for_each(|l| out.insert(out.len(), ListItem::new(l.to_string())));

            container.info = StatefulList::new(out)
        }
    }

    pub fn get_infos(&mut self) -> Vec<ListItem<'static>> {
        self.containers
            .state
            .selected()
            .and_then(|i| self.containers.items.get_mut(i))
            .map_or(vec![], |i| i.info.items.clone())
    }

    /// Update, or insert, containers
    pub fn update_containers(&mut self, all_containers: &mut [ContainerSummary]) {
        let all_ids = self
            .containers
            .items
            .iter()
            .map(|i| i.id.clone())
            .collect::<Vec<_>>();

        // Only sort it no containers currently set, as afterwards the order is fixed
        if self.containers.items.is_empty() {
            all_containers.sort_by(|a, b| a.created.cmp(&b.created));
        }

        if !all_containers.is_empty() && self.containers.state.selected().is_none() {
            self.containers.start();
        }

        for (index, id) in all_ids.iter().enumerate() {
            if !all_containers
                .iter()
                .filter_map(|i| i.id.as_ref())
                .any(|x| x == id.get())
            {
                // If removed container is currently selected, then change selected to previous
                // This will default to 0 in any edge cases
                if self.containers.state.selected().is_some() {
                    self.containers.previous();
                }
                // Check is some, else can cause out of bounds error, if containers get removed before a docker update
                if self.containers.items.get(index).is_some() {
                    self.containers.items.remove(index);
                }
            }
        }

        // Trim a &String and return String
        let trim_owned = |x: &String| x.trim().to_owned();

        for i in all_containers {
            if let Some(id) = i.id.as_ref() {
                let name = i.names.as_mut().map_or(String::new(), |names| {
                    names.first_mut().map_or(String::new(), |f| {
                        if f.starts_with('/') {
                            f.remove(0);
                        }
                        (*f).to_string()
                    })
                });

                let is_oxker = i
                    .command
                    .as_ref()
                    .map_or(false, |i| i.starts_with(ENTRY_POINT));

                let state = State::from(i.state.as_ref().map_or("dead".to_owned(), trim_owned));
                let status = i.status.as_ref().map_or(String::new(), trim_owned);

                let image = i
                    .image
                    .as_ref()
                    .map_or(String::new(), std::clone::Clone::clone);

                let id = ContainerId::from(id);

                let created = i
                    .created
                    .map_or(0, |i| u64::try_from(i).unwrap_or_default());
                // If container info already in containers Vec, then just update details
                if let Some(item) = self.get_container_by_id(&id) {
                    if item.name != name {
                        item.name = name;
                    };
                    if item.status != status {
                        item.status = status;
                    };
                    if item.state != state {
                        item.state = state;
                    };
                    if item.image != image {
                        item.image = image;
                    };
                } else {
                    // container not known, so make new ContainerItem and push into containers Vec
                    let container =
                        ContainerItem::new(created, id, image, is_oxker, name, state, status);
                    self.containers.items.push(container);
                }
            }
        }
    }

    /// Current time as unix timestamp
    #[allow(clippy::expect_used)]
    fn get_systemtime() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("In our known reality, this error should never occur")
            .as_secs()
    }

    /// update logs of a given container, based on id
    pub fn update_log_by_id(&mut self, logs: Vec<String>, id: &ContainerId) {
        let color = self.args.color;
        let raw = self.args.raw;

        let timestamp = self.args.timestamp;

        if let Some(container) = self.get_container_by_id(id) {
            container.last_updated = Self::get_systemtime();
            let current_len = container.logs.len();

            for mut i in logs {
                let tz = LogsTz::from(&i);
                // Strip the timestamp if `-t` flag set
                if !timestamp {
                    i = i.replace(&tz.to_string(), "");
                }
                let lines = if color {
                    log_sanitizer::colorize_logs(&i)
                } else if raw {
                    log_sanitizer::raw(&i)
                } else {
                    log_sanitizer::remove_ansi(&i)
                };
                container.logs.insert(ListItem::new(lines), tz);
            }

            // Set the logs selected row for each container
            // Either when no long currently selected, or currently selected (before updated) is already at end
            if container.logs.state().selected().is_none()
                || container.logs.state().selected().map_or(1, |f| f + 1) == current_len
            {
                container.logs.end();
            }
        }
    }
}
