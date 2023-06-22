use std::{
    cmp::Ordering,
    collections::{HashSet, VecDeque},
    fmt,
};

use ratatui::{
    style::Color,
    widgets::{ListItem, ListState},
};

use crate::app_data::container_data::Header;

const ONE_KB: f64 = 1000.0;
const ONE_MB: f64 = ONE_KB * 1000.0;
const ONE_GB: f64 = ONE_MB * 1000.0;

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct ContainerId(String);

impl From<String> for ContainerId {
    fn from(x: String) -> Self {
        Self(x)
    }
}

impl From<&String> for ContainerId {
    fn from(x: &String) -> Self {
        Self(x.clone())
    }
}

impl From<&str> for ContainerId {
    fn from(x: &str) -> Self {
        Self(x.to_owned())
    }
}

impl ContainerId {
    pub fn get(&self) -> &str {
        self.0.as_str()
    }
}

impl Ord for ContainerId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for ContainerId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone)]
pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
}

impl<T> StatefulList<T> {
    pub fn new(items: Vec<T>) -> Self {
        Self {
            state: ListState::default(),
            items,
        }
    }

    pub fn end(&mut self) {
        let len = self.items.len();
        if len > 0 {
            self.state.select(Some(self.items.len() - 1));
        }
    }

    pub fn start(&mut self) {
        self.state.select(Some(0));
    }

    pub fn next(&mut self) {
        if !self.items.is_empty() {
            let i = match self.state.selected() {
                Some(i) => {
                    if i < self.items.len() - 1 {
                        i + 1
                    } else {
                        i
                    }
                }
                None => 0,
            };
            self.state.select(Some(i));
        }
    }

    pub fn previous(&mut self) {
        if !self.items.is_empty() {
            let i = self
                .state
                .selected()
                .map_or(0, |i| if i == 0 { 0 } else { i - 1 });
            self.state.select(Some(i));
        }
    }

    /// Return the current status of the select list, e.g. 2/5,
    pub fn get_state_title(&self) -> String {
        if self.items.is_empty() {
            String::new()
        } else {
            let len = self.items.len();
            let c = self
                .state
                .selected()
                .map_or(0, |value| if len > 0 { value + 1 } else { value });
            format!("{c}/{}", self.items.len())
        }
    }
}

/// States of the container
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd)]
pub enum State {
    Dead,
    Exited,
    Paused,
    Removing,
    Restarting,
    Running,
    Unknown,
}

impl State {
    pub const fn get_color(self) -> Color {
        match self {
            Self::Paused => Color::Yellow,
            Self::Removing => Color::LightRed,
            Self::Restarting => Color::LightGreen,
            Self::Running => Color::Green,
            _ => Color::Red,
        }
    }
    /// Dirty way to create order for the state, rather than impl Ord
    pub const fn order(self) -> u8 {
        match self {
            Self::Running => 0,
            Self::Paused => 1,
            Self::Restarting => 2,
            Self::Removing => 3,
            Self::Exited => 4,
            Self::Dead => 5,
            Self::Unknown => 6,
        }
    }
}

impl From<String> for State {
    fn from(input: String) -> Self {
        match input.as_ref() {
            "dead" => Self::Dead,
            "exited" => Self::Exited,
            "paused" => Self::Paused,
            "removing" => Self::Removing,
            "restarting" => Self::Restarting,
            "running" => Self::Running,
            _ => Self::Unknown,
        }
    }
}

impl From<&str> for State {
    fn from(input: &str) -> Self {
        match input {
            "dead" => Self::Dead,
            "exited" => Self::Exited,
            "paused" => Self::Paused,
            "removing" => Self::Removing,
            "restarting" => Self::Restarting,
            "running" => Self::Running,
            _ => Self::Unknown,
        }
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let disp = match self {
            Self::Dead => "✖ dead",
            Self::Exited => "✖ exited",
            Self::Paused => "॥ paused",
            Self::Removing => "removing",
            Self::Restarting => "↻ restarting",
            Self::Running => "✓ running",
            Self::Unknown => "? unknown",
        };
        write!(f, "{disp}")
    }
}

/// Items for the container control list
#[derive(Debug, Clone, Copy)]
pub enum DockerControls {
    Pause,
    Restart,
    Start,
    Stop,
    Unpause,
    Delete,
}

impl DockerControls {
    pub const fn get_color(self) -> Color {
        match self {
            Self::Pause => Color::Yellow,
            Self::Restart => Color::Magenta,
            Self::Start => Color::Green,
            Self::Stop => Color::Red,
            Self::Delete => Color::Gray,
            Self::Unpause => Color::Blue,
        }
    }

    /// Docker commands available depending on the containers state
    pub fn gen_vec(state: State) -> Vec<Self> {
        match state {
            State::Dead | State::Exited => vec![Self::Start, Self::Restart, Self::Delete],
            State::Paused => vec![Self::Unpause, Self::Stop, Self::Delete],
            State::Restarting => vec![Self::Stop, Self::Delete],
            State::Running => vec![Self::Pause, Self::Restart, Self::Stop, Self::Delete],
            _ => vec![Self::Delete],
        }
    }
}

impl fmt::Display for DockerControls {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let disp = match self {
            Self::Pause => "pause",
            Self::Delete => "delete",
            Self::Restart => "restart",
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Unpause => "unpause",
        };
        write!(f, "{disp}")
    }
}

pub trait Stats {
    fn get_value(&self) -> f64;
}

/// Struct for frequently updated CPU stats
/// So can use custom display formatter
/// Use trait Stats for use as generic in draw_chart function
#[derive(Debug, Default, Clone, Copy)]
pub struct CpuStats(f64);

impl CpuStats {
    pub const fn new(value: f64) -> Self {
        Self(value)
    }
}

impl Eq for CpuStats {}

impl PartialEq for CpuStats {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl PartialOrd for CpuStats {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for CpuStats {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.0 > other.0 {
            Ordering::Greater
        } else if (self.0 - other.0).abs() < 0.01 {
            Ordering::Equal
        } else {
            Ordering::Less
        }
    }
}

impl Stats for CpuStats {
    fn get_value(&self) -> f64 {
        self.0
    }
}

impl fmt::Display for CpuStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let disp = format!("{:05.2}%", self.0);
        write!(f, "{disp:>x$}", x = f.width().unwrap_or(1))
    }
}

/// Struct for frequently updated memory usage stats
/// So can use custom display formatter
/// Use trait Stats for use as generic in draw_chart function
#[derive(Debug, Default, Clone, Copy, Eq)]
pub struct ByteStats(u64);

impl PartialEq for ByteStats {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl PartialOrd for ByteStats {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for ByteStats {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl ByteStats {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub fn update(&mut self, value: u64) {
        self.0 = value;
    }
}

#[allow(clippy::cast_precision_loss)]
impl Stats for ByteStats {
    fn get_value(&self) -> f64 {
        self.0 as f64
    }
}

/// convert from bytes to kB, MB, GB etc
impl fmt::Display for ByteStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let as_f64 = self.get_value();
        let p = match as_f64 {
            x if x >= ONE_GB => format!("{y:.2} GB", y = as_f64 / ONE_GB),
            x if x >= ONE_MB => format!("{y:.2} MB", y = as_f64 / ONE_MB),
            _ => format!("{y:.2} kB", y = as_f64 / ONE_KB),
        };
        write!(f, "{p:>x$}", x = f.width().unwrap_or(1))
    }
}

pub type MemTuple = (Vec<(f64, f64)>, ByteStats, State);
pub type CpuTuple = (Vec<(f64, f64)>, CpuStats, State);

/// Used to make sure that each log entry, for each container, is unique,
/// will only push a log entry into the logs vec if timetstamp of said log entry isn't in the hashset
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct LogsTz(String);

/// The docker log, which should always contain a timestamp, is in the format `2023-01-14T19:13:30.783138328Z Lorem ipsum dolor sit amet`
/// So just split at the inclusive index of the first space, needs to be inclusive, hence the use of format to at the space, so that we can remove the whole thing when the `-t` flag is set
/// Need to make sure that this isn't an empty string?!
impl From<&String> for LogsTz {
    fn from(value: &String) -> Self {
        Self(value.split_inclusive(' ').take(1).collect::<String>())
    }
}

impl fmt::Display for LogsTz {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Store the logs alongside a HashSet, each log *should* generate a unique timestamp,
/// so if we store the timestamp separately in a HashSet, we can then check if we should insert a log line into the
/// stateful list dependent on whethere the timestamp is in the HashSet or not
#[derive(Debug, Clone)]
pub struct Logs {
    logs: StatefulList<ListItem<'static>>,
    tz: HashSet<LogsTz>,
}

impl Default for Logs {
    fn default() -> Self {
        let mut logs = StatefulList::new(vec![]);
        logs.end();
        Self {
            logs,
            tz: HashSet::new(),
        }
    }
}

impl Logs {
    /// Only allow a new log line to be inserted if the log timestamp isn't in the tz HashSet
    pub fn insert(&mut self, line: ListItem<'static>, tz: LogsTz) {
        if self.tz.insert(tz) {
            self.logs.items.push(line);
        };
    }

    pub fn to_vec(&self) -> Vec<ListItem<'static>> {
        self.logs.items.clone()
    }

    /// The rest of the methods are basically forwarding from the underlying StatefulList
    pub fn get_state_title(&self) -> String {
        self.logs.get_state_title()
    }

    pub fn next(&mut self) {
        self.logs.next();
    }

    pub fn previous(&mut self) {
        self.logs.previous();
    }

    pub fn end(&mut self) {
        self.logs.end();
    }
    pub fn start(&mut self) {
        self.logs.start();
    }

    pub fn len(&self) -> usize {
        self.logs.items.len()
    }

    pub fn state(&mut self) -> &mut ListState {
        &mut self.logs.state
    }
}

/// Info for each container
#[derive(Debug, Clone)]
pub struct ContainerItem {
    pub created: u64,
    pub cpu_stats: VecDeque<CpuStats>,
    pub docker_controls: StatefulList<DockerControls>,
    pub id: ContainerId,
    pub image: String,
    pub last_updated: u64,
    pub logs: Logs,
    pub mem_limit: ByteStats,
    pub mem_stats: VecDeque<ByteStats>,
    pub name: String,
    pub rx: ByteStats,
    pub state: State,
    pub status: String,
    pub tx: ByteStats,
    pub is_oxker: bool,
}

impl ContainerItem {
    /// Create a new container item
    pub fn new(
        created: u64,
        id: ContainerId,
        image: String,
        is_oxker: bool,
        name: String,
        state: State,
        status: String,
    ) -> Self {
        let mut docker_controls = StatefulList::new(DockerControls::gen_vec(state));
        docker_controls.start();
        Self {
            created,
            cpu_stats: VecDeque::with_capacity(60),
            docker_controls,
            id,
            image,
            is_oxker,
            last_updated: 0,
            logs: Logs::default(),
            mem_limit: ByteStats::default(),
            mem_stats: VecDeque::with_capacity(60),
            name,
            rx: ByteStats::default(),
            state,
            status,
            tx: ByteStats::default(),
        }
    }

    /// Find the max value in the cpu stats VecDeque
    fn max_cpu_stats(&self) -> CpuStats {
        self.cpu_stats
            .iter()
            .max()
            .map_or_else(CpuStats::default, |value| *value)
    }

    /// Find the max value in the mem stats VecDeque
    fn max_mem_stats(&self) -> ByteStats {
        self.mem_stats
            .iter()
            .max()
            .map_or_else(ByteStats::default, |value| *value)
    }

    /// Convert cpu stats into a vec for the charts function
    #[allow(clippy::cast_precision_loss)]
    fn get_cpu_dataset(&self) -> Vec<(f64, f64)> {
        self.cpu_stats
            .iter()
            .enumerate()
            .map(|i| (i.0 as f64, i.1 .0))
            .collect::<Vec<_>>()
    }

    /// Convert mem stats into a Vec for the charts function
    #[allow(clippy::cast_precision_loss)]
    fn get_mem_dataset(&self) -> Vec<(f64, f64)> {
        self.mem_stats
            .iter()
            .enumerate()
            .map(|i| (i.0 as f64, i.1 .0 as f64))
            .collect::<Vec<_>>()
    }

    /// Get all cpu chart data
    fn get_cpu_chart_data(&self) -> CpuTuple {
        (self.get_cpu_dataset(), self.max_cpu_stats(), self.state)
    }

    /// Get all mem chart data
    fn get_mem_chart_data(&self) -> MemTuple {
        (self.get_mem_dataset(), self.max_mem_stats(), self.state)
    }

    /// Get chart info for cpu & memory in one function
    /// So only need to call .lock() once
    pub fn get_chart_data(&self) -> (CpuTuple, MemTuple) {
        (self.get_cpu_chart_data(), self.get_mem_chart_data())
    }
}

/// Container information panel headings + widths, for nice pretty formatting
#[derive(Debug, Clone, Copy)]
pub struct Columns {
    pub state: (Header, u8),
    pub status: (Header, u8),
    pub cpu: (Header, u8),
    pub mem: (Header, u8, u8),
    pub id: (Header, u8),
    pub name: (Header, u8),
    pub image: (Header, u8),
    pub net_rx: (Header, u8),
    pub net_tx: (Header, u8),
}

impl Columns {
    /// (Column titles, minimum header string length)
    pub const fn new() -> Self {
        Self {
            state: (Header::State, 11),
            status: (Header::Status, 16),
            cpu: (Header::Cpu, 7),
            mem: (Header::Memory, 7, 7),
            id: (Header::Id, 8),
            name: (Header::Name, 4),
            image: (Header::Image, 5),
            net_rx: (Header::Rx, 7),
            net_tx: (Header::Tx, 7),
        }
    }
}
