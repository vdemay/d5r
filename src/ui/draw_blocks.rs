use parking_lot::Mutex;
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{
        Axis, Block, BorderType, Borders, Chart, Clear, Dataset, GraphType, List, ListItem,
        Paragraph,
    },
    Frame,
};
use std::default::Default;
use std::{fmt::Display, sync::Arc};
use tracing::Instrument;

use crate::app_data::{Header, SortedOrder};
use crate::ui::Status;
use crate::{
    app_data::{AppData, ByteStats, Columns, CpuStats, State, Stats},
    app_error::AppError,
};
use crate::ui::gui_state::NavPanel;

use super::gui_state::{BoxLocation, DeleteButton, Region};
use super::{GuiState, SelectablePanel};

const LOGO: &str = r#"    .___.________
  __| _/|   ____/______
 / __ | |____  \\_  __ \
/ /_/ | /       \|  | \/
\____ |/______  /|__|
     \/       \/        "#;

const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const REPO: &str = env!("CARGO_PKG_REPOSITORY");
const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
const ORANGE: Color = Color::Rgb(255, 178, 36);
const MARGIN: &str = "   ";
const ARROW: &str = "▶ ";
const CIRCLE: &str = "* ";

/// From a given &str, return the maximum number of chars on a single line
fn max_line_width(text: &str) -> usize {
    text.lines()
        .map(|i| i.chars().count())
        .max()
        .unwrap_or_default()
}

/// Generate block, add a border if is the selected panel,
/// add custom title based on state of each panel
fn generate_block<'a>(
    app_data: &Arc<Mutex<AppData>>,
    area: Rect,
    gui_state: &Arc<Mutex<GuiState>>,
    panel: SelectablePanel,
) -> Block<'a> {
    gui_state
        .lock()
        .update_region_map(Region::Panel(panel), area);
    let current_selected_panel = gui_state.lock().selected_panel;
    let mut title = match panel {
        SelectablePanel::Containers => {
            format!("{} {}", panel.title(), app_data.lock().container_title())
        }
        SelectablePanel::Logs => {
            format!("{} {}", panel.title(), app_data.lock().get_log_title())
        }
        SelectablePanel::Commands => String::new(),
    };
    if !title.is_empty() {
        title = format!(" {title} ");
    }
    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title);
    if current_selected_panel == panel {
        block = block.border_style(Style::default().fg(Color::LightCyan));
    }
    block
}

/// Draw the command panel
pub fn commands<B: Backend>(
    app_data: &Arc<Mutex<AppData>>,
    area: Rect,
    f: &mut Frame<'_, B>,
    gui_state: &Arc<Mutex<GuiState>>,
) {
    let block = || generate_block(app_data, area, gui_state, SelectablePanel::Commands);
    let items = app_data.lock().get_control_items().map_or(vec![], |i| {
        i.iter()
            .map(|c| {
                let lines = Line::from(vec![Span::styled(
                    c.to_string(),
                    Style::default().fg(c.get_color()),
                )]);
                ListItem::new(lines)
            })
            .collect::<Vec<_>>()
    });

    let items = List::new(items)
        .block(block())
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(ARROW);

    if let Some(i) = app_data.lock().get_control_state() {
        f.render_stateful_widget(items, area, i);
    } else {
        let paragraph = Paragraph::new("")
            .block(block())
            .alignment(Alignment::Center);
        f.render_widget(paragraph, area);
    }
}

/// Draw the containers panel
pub fn containers<B: Backend>(
    app_data: &Arc<Mutex<AppData>>,
    area: Rect,
    f: &mut Frame<'_, B>,
    gui_state: &Arc<Mutex<GuiState>>,
    widths: &Columns,
) {
    let block = generate_block(app_data, area, gui_state, SelectablePanel::Containers);

    let items = app_data
        .lock()
        .get_container_items()
        .iter()
        .map(|i| {
            let state_style = Style::default().fg(i.state.get_color());
            let blue = Style::default().fg(Color::Blue);

            let lines = Line::from(vec![
                Span::styled(
                    format!(
                        "{:<width$}",
                        i.state.to_string(),
                        width = widths.state.1.into()
                    ),
                    state_style,
                ),
                Span::styled(
                    format!(
                        "{MARGIN}{:>width$}",
                        i.status,
                        width = &widths.status.1.into()
                    ),
                    state_style,
                ),
                Span::styled(
                    format!(
                        "{}{:>width$}",
                        MARGIN,
                        i.cpu_stats.back().unwrap_or(&CpuStats::default()),
                        width = &widths.cpu.1.into()
                    ),
                    state_style,
                ),
                Span::styled(
                    format!(
                        "{MARGIN}{:>width_current$} / {:>width_limit$}",
                        i.mem_stats.back().unwrap_or(&ByteStats::default()),
                        i.mem_limit,
                        width_current = &widths.mem.1.into(),
                        width_limit = &widths.mem.2.into()
                    ),
                    state_style,
                ),
                Span::styled(
                    format!(
                        "{}{:>width$}",
                        MARGIN,
                        i.id.get().chars().take(8).collect::<String>(),
                        width = &widths.id.1.into()
                    ),
                    blue,
                ),
                Span::styled(
                    format!("{MARGIN}{:>width$}", i.name, width = widths.name.1.into()),
                    blue,
                ),
                Span::styled(
                    format!("{MARGIN}{:>width$}", i.image, width = widths.image.1.into()),
                    blue,
                ),
                Span::styled(
                    format!("{MARGIN}{:>width$}", i.rx, width = widths.net_rx.1.into()),
                    Style::default().fg(Color::Rgb(255, 233, 193)),
                ),
                Span::styled(
                    format!("{MARGIN}{:>width$}", i.tx, width = widths.net_tx.1.into()),
                    Style::default().fg(Color::Rgb(205, 140, 140)),
                ),
            ]);
            ListItem::new(lines)
        })
        .collect::<Vec<_>>();

    if items.is_empty() {
        let paragraph = Paragraph::new("no containers running")
            .block(block)
            .alignment(Alignment::Center);
        f.render_widget(paragraph, area);
    } else {
        let items = List::new(items)
            .block(block)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD).bg(Color::Blue))
            .highlight_symbol(CIRCLE);

        f.render_stateful_widget(items, area, app_data.lock().get_container_state());
    }
}

/// Draw the logs panel
pub fn logs<B: Backend>(
    app_data: &Arc<Mutex<AppData>>,
    area: Rect,
    f: &mut Frame<'_, B>,
    gui_state: &Arc<Mutex<GuiState>>,
    loading_icon: &str,
) {
    let block = || generate_block(app_data, area, gui_state, SelectablePanel::Logs);
    if gui_state.lock().status_contains(&[Status::Init]) {
        let paragraph = Paragraph::new(format!("parsing logs {loading_icon}"))
            .style(Style::default())
            .block(block())
            .alignment(Alignment::Center);
        f.render_widget(paragraph, area);
    } else {
        let logs = app_data.lock().get_logs();

        if logs.is_empty() {
            let paragraph = Paragraph::new("no logs found")
                .block(block())
                .alignment(Alignment::Center);
            f.render_widget(paragraph, area);
        } else {
            let items = List::new(logs)
                .block(block())
                .highlight_symbol(ARROW)
                .highlight_style(Style::default().add_modifier(Modifier::BOLD));

            // This should always return Some, as logs is not empty
            if let Some(i) = app_data.lock().get_log_state() {
                f.render_stateful_widget(items, area, i);
            }
        }
    }
}

/// Draw the cpu + mem charts
pub fn chart<B: Backend>(f: &mut Frame<'_, B>, area: Rect, app_data: &Arc<Mutex<AppData>>) {
    if let Some((cpu, mem)) = app_data.lock().get_chart_data() {
        let area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(area);

        let cpu_dataset = vec![Dataset::default()
            .marker(symbols::Marker::Dot)
            .style(Style::default().fg(Color::Magenta))
            .graph_type(GraphType::Line)
            .data(&cpu.0)];
        let mem_dataset = vec![Dataset::default()
            .marker(symbols::Marker::Dot)
            .style(Style::default().fg(Color::Cyan))
            .graph_type(GraphType::Line)
            .data(&mem.0)];

        let cpu_stats = CpuStats::new(cpu.0.last().map_or(0.00, |f| f.1));
        let mem_stats = ByteStats::new(mem.0.last().map_or(0, |f| f.1 as u64));
        let cpu_chart = make_chart(cpu.2, "cpu", cpu_dataset, &cpu_stats, &cpu.1);
        let mem_chart = make_chart(mem.2, "memory", mem_dataset, &mem_stats, &mem.1);

        f.render_widget(cpu_chart, area[0]);
        f.render_widget(mem_chart, area[1]);
    }
}

/// Create charts
fn make_chart<'a, T: Stats + Display>(
    state: State,
    name: &'a str,
    dataset: Vec<Dataset<'a>>,
    current: &'a T,
    max: &'a T,
) -> Chart<'a> {
    let title_color = match state {
        State::Running => Color::Green,
        _ => state.get_color(),
    };
    let label_color = match state {
        State::Running => ORANGE,
        _ => state.get_color(),
    };
    Chart::new(dataset)
        .block(
            Block::default()
                .title_alignment(Alignment::Center)
                .title(Span::styled(
                    format!(" {name} {current} "),
                    Style::default()
                        .fg(title_color)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .x_axis(
            Axis::default()
                .style(Style::default().fg(title_color))
                .bounds([0.00, 60.0]),
        )
        .y_axis(
            Axis::default()
                .labels(vec![
                    Span::styled("", Style::default().fg(label_color)),
                    Span::styled(
                        format!("{max}"),
                        Style::default()
                            .add_modifier(Modifier::BOLD)
                            .fg(label_color),
                    ),
                ])
                // Add 0.01, so that max point is always visible?
                .bounds([0.0, max.get_value() + 0.01]),
        )
}

pub fn top_menu<B: Backend>(
    f: &mut Frame<'_, B>,
    area: Rect,
    gui_state: &Arc<Mutex<GuiState>>,
) {
    let split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(30), Constraint::Percentage(100), Constraint::Min(25)].as_ref())
        .split(area);

    // left part
    let mut left_lines = vec![
        Line::from(""),
        Line::from(Span::styled("Hackathon", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled("June 2023", Style::default().fg(Color::White))),
    ];
    let left = Paragraph::new(left_lines)
        .style(Style::default().fg(Color::White))
        .block(Block::default().style(Style::default()).borders(Borders::NONE))
        .alignment(Alignment::Left);
    f.render_widget(left, split[0]);

    //actions
    let mut actions_lines = vec![
        Line::from(""),
    ];
    match gui_state.lock().current_panel {
        NavPanel::Containers => {
            actions_lines.insert(actions_lines.len(), Line::from(Span::styled("(s) start", Style::default().fg(Color::White))));
            actions_lines.insert(actions_lines.len(), Line::from(Span::styled("(S) stop", Style::default().fg(Color::White))));
            actions_lines.insert(actions_lines.len(), Line::from(Span::styled("(r) restart", Style::default().fg(Color::White))));
        }
        NavPanel::Logs{container_name: _} => {
            actions_lines.insert(actions_lines.len(), Line::from(Span::styled("(s) toggle scroll", Style::default().fg(Color::White))));
        }
    };
    let actions = Paragraph::new(actions_lines)
        .style(Style::default().fg(Color::White))
        .block(Block::default().style(Style::default()).borders(Borders::NONE))
        .alignment(Alignment::Left);
    f.render_widget(actions, split[1]);

    // Right logo drawing
    let mut logo_lines = LOGO
        .lines()
        .map(|i| Line::from(Span::styled(i.to_owned(), Style::default().fg(Color::White))))
        .collect::<Vec<_>>();
    let logo = Paragraph::new(logo_lines)
        .style(Style::default().fg(Color::White))
        .block(Block::default().style(Style::default()).borders(Borders::NONE))
        .alignment(Alignment::Left);
    f.render_widget(logo, split[2]);
}

/// Draw heading bar at top of program, always visible
/// TODO Should separate into loading icon/headers/help functions
#[allow(clippy::too_many_lines)]
pub fn heading_bar<B: Backend>(
    area: Rect,
    columns: &Columns,
    f: &mut Frame<'_, B>,
    has_containers: bool,
    loading_icon: &str,
    sorted_by: Option<(Header, SortedOrder)>,
    gui_state: &Arc<Mutex<GuiState>>,
) {
    let block = |fg: Color| Block::default().style(Style::default().bg(Color::Magenta).fg(fg));
    let help_visible = gui_state.lock().status_contains(&[Status::Help]);

    f.render_widget(block(Color::Black), area);

    // Generate a block for the header, if the header is currently being used to sort a column, then highlight it white
    let header_block = |x: &Header| {
        let mut color = Color::Black;
        let mut suffix = "";
        let mut suffix_margin = 0;
        if let Some((a, b)) = sorted_by.as_ref() {
            if x == a {
                match b {
                    SortedOrder::Asc => suffix = " ⌃",
                    SortedOrder::Desc => suffix = " ⌄",
                }
                suffix_margin = 2;
                color = Color::White;
            };
        };
        (
            Block::default().style(Style::default().bg(Color::Magenta).fg(color)),
            suffix,
            suffix_margin,
        )
    };

    // Generate block for the headers, state and status has a specific layout, others all equal
    // width is dependant on it that column is selected to sort - or not
    let gen_header = |header: &Header, width: usize| {
        let block = header_block(header);
        let text = match header {
            Header::State => format!(
                "{:>width$}{ic}",
                header,
                ic = block.1,
                width = width - block.2,
            ),
            Header::Status => format!(
                "{}  {:>width$}{ic}",
                MARGIN,
                header,
                ic = block.1,
                width = width - block.2
            ),
            _ => format!(
                "{}{:>width$}{ic}",
                MARGIN,
                header,
                ic = block.1,
                width = width - block.2
            ),
        };
        let count = u16::try_from(text.chars().count()).unwrap_or_default();
        let status = Paragraph::new(text)
            .block(block.0)
            .alignment(Alignment::Left);
        (status, count)
    };

    // Meta data to iterate over to create blocks with correct widths
    let header_meta = [
        (Header::State, columns.state.1),
        (Header::Status, columns.status.1),
        (Header::Cpu, columns.cpu.1),
        (Header::Memory, columns.mem.1 + columns.mem.2 + 3),
        (Header::Id, columns.id.1),
        (Header::Name, columns.name.1),
        (Header::Image, columns.image.1),
        (Header::Rx, columns.net_rx.1),
        (Header::Tx, columns.net_tx.1),
    ];

    let header_data = header_meta
        .iter()
        .map(|i| {
            let header_block = gen_header(&i.0, i.1.into());
            (header_block.0, i.0, Constraint::Max(header_block.1))
        })
        .collect::<Vec<_>>();

    let suffix = if help_visible { "exit" } else { "show" };
    let info_text = format!("( h ) {suffix} help {MARGIN}",);
    let info_width = info_text.chars().count();

    let column_width = usize::from(area.width).saturating_sub(info_width);
    let column_width = if column_width > 0 { column_width } else { 1 };
    let splits = if has_containers {
        vec![
            Constraint::Min(2),
            Constraint::Min(column_width.try_into().unwrap_or_default()),
            Constraint::Min(info_width.try_into().unwrap_or_default()),
        ]
    } else {
        vec![Constraint::Percentage(100)]
    };

    let split_bar = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(splits.as_ref())
        .split(area);
    if has_containers {
        // Draw loading icon, or not, and a prefix with a single space
        let loading_icon = format!("{loading_icon:>2}");
        let loading_paragraph = Paragraph::new(loading_icon)
            .block(block(Color::White))
            .alignment(Alignment::Center);
        f.render_widget(loading_paragraph, split_bar[0]);

        let container_splits = header_data.iter().map(|i| i.2).collect::<Vec<_>>();
        let headers_section = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(container_splits.as_ref())
            .split(split_bar[1]);

        // draw the actual header blocks
        for (index, (paragraph, header, _)) in header_data.into_iter().enumerate() {
            let rect = headers_section[index];
            gui_state
                .lock()
                .update_region_map(Region::Header(header), rect);
            f.render_widget(paragraph, rect);
        }
    }

    // show/hide help
    let color = if help_visible {
        Color::Black
    } else {
        Color::White
    };
    let help_paragraph = Paragraph::new(info_text)
        .block(block(color))
        .alignment(Alignment::Right);

    // If no containers, don't display the headers, could maybe do this first?
    let help_index = if has_containers { 2 } else { 0 };
    f.render_widget(help_paragraph, split_bar[help_index]);
}

/// Help popup box needs these three pieces of information
struct HelpInfo {
    lines: Vec<Line<'static>>,
    width: usize,
    height: usize,
}

impl HelpInfo {
    /// Find the max width of a Span in &[Line], although it isn't calculating it correctly
    fn calc_width(lines: &[Line]) -> usize {
        lines
            .iter()
            .flat_map(|x| x.spans.iter())
            .map(ratatui::text::Span::width)
            .max()
            .unwrap_or(1)
    }

    /// Just an empty span, i.e. a new line
    fn empty_span<'a>() -> Line<'a> {
        Line::from(String::new())
    }

    /// generate a span, of given &str and given color
    fn span<'a>(input: &str, color: Color) -> Span<'a> {
        Span::styled(input.to_owned(), Style::default().fg(color))
    }

    /// &str to black text span
    fn black_span<'a>(input: &str) -> Span<'a> {
        Self::span(input, Color::Black)
    }

    /// &str to white text span
    fn white_span<'a>(input: &str) -> Span<'a> {
        Self::span(input, Color::White)
    }

    /// Generate the `oxker` name span + metadata
    fn gen_name() -> Self {
        let mut lines = LOGO
            .lines()
            .map(|i| Line::from(Self::white_span(i)))
            .collect::<Vec<_>>();
        lines.insert(0, Self::empty_span());
        let width = Self::calc_width(&lines);
        let height = lines.len();
        Self {
            lines,
            width,
            height,
        }
    }

    /// Generate the description span + metadata
    fn gen_description() -> Self {
        let lines = [
            Self::empty_span(),
            Line::from(Self::white_span(DESCRIPTION)),
            Self::empty_span(),
        ];
        let width = Self::calc_width(&lines);
        let height = lines.len();
        Self {
            lines: lines.to_vec(),
            width,
            height,
        }
    }

    /// Generate the button information span + metadata
    fn gen_button() -> Self {
        let button_item = |x: &str| Self::white_span(&format!(" ( {x} ) "));
        let button_desc = |x: &str| Self::black_span(x);
        let or = || button_desc("or");
        let space = || button_desc(" ");

        let lines = [
            Line::from(vec![
                space(),
                button_item("tab"),
                or(),
                button_item("shift+tab"),
                button_desc("to change panels"),
            ]),
            Line::from(vec![
                space(),
                button_item("↑ ↓"),
                or(),
                button_item("j k"),
                or(),
                button_item("PgUp PgDown"),
                or(),
                button_item("Home End"),
                button_desc("to change selected line"),
            ]),
            Line::from(vec![
                space(),
                button_item("enter"),
                button_desc("to send docker container command"),
            ]),
            Line::from(vec![
                space(),
                button_item("h"),
                button_desc("to toggle this help information"),
            ]),
            Line::from(vec![space(), button_item("0"), button_desc("to stop sort")]),
            Line::from(vec![
                space(),
                button_item("1 - 9"),
                button_desc("sort by header - or click header"),
            ]),
            Line::from(vec![
				space(),
				button_item("m"),
				button_desc(
					"to toggle mouse capture - if disabled, text on screen can be selected & copied",
				),
			]),
            Line::from(vec![
                space(),
                button_item("q"),
                button_desc("to quit at any time"),
            ]),
        ];

        let height = lines.len();
        let width = Self::calc_width(&lines);
        Self {
            lines: lines.to_vec(),
            width,
            height,
        }
    }

    /// Generate the final lines, GitHub link etc, + metadata
    fn gen_final() -> Self {
        let lines = [
            Self::empty_span(),
            Line::from(vec![Self::black_span(
                "currently an early work in progress, all and any input appreciated",
            )]),
            Line::from(vec![Span::styled(
                REPO.to_owned(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::UNDERLINED),
            )]),
        ];
        let height = lines.len();
        let width = Self::calc_width(&lines);
        Self {
            lines: lines.to_vec(),
            width,
            height,
        }
    }
}

/// Draw the help box in the centre of the screen
pub fn help_box<B: Backend>(f: &mut Frame<'_, B>) {
    let title = format!(" {VERSION} ");

    let name_info = HelpInfo::gen_name();
    let description_info = HelpInfo::gen_description();
    let button_info = HelpInfo::gen_button();
    let final_info = HelpInfo::gen_final();

    // have to add 10, but shouldn't need to, is an error somewhere
    let max_line_width = [
        name_info.width,
        description_info.width,
        button_info.width,
        final_info.width,
    ]
    .into_iter()
    .max()
    .unwrap_or_default()
        + 10;
    let max_height =
        name_info.height + description_info.height + button_info.height + final_info.height + 2;

    let area = popup(
        max_height,
        max_line_width,
        f.size(),
        BoxLocation::MiddleCentre,
    );

    let split_popup = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Max(name_info.height.try_into().unwrap_or_default()),
                Constraint::Max(description_info.height.try_into().unwrap_or_default()),
                Constraint::Max(button_info.height.try_into().unwrap_or_default()),
                Constraint::Max(final_info.height.try_into().unwrap_or_default()),
            ]
            .as_ref(),
        )
        .split(area);

    let name_paragraph = Paragraph::new(name_info.lines)
        .style(Style::default().bg(Color::Magenta).fg(Color::White))
        .block(Block::default())
        .alignment(Alignment::Left);

    let description_paragraph = Paragraph::new(description_info.lines)
        .style(Style::default().bg(Color::Magenta).fg(Color::Black))
        .block(Block::default())
        .alignment(Alignment::Center);

    let help_paragraph = Paragraph::new(button_info.lines)
        .style(Style::default().bg(Color::Magenta).fg(Color::Black))
        .block(Block::default())
        .alignment(Alignment::Left);

    let final_paragraph = Paragraph::new(final_info.lines)
        .style(Style::default().bg(Color::Magenta).fg(Color::Black))
        .block(Block::default())
        .alignment(Alignment::Center);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Black));

    // Order is important here
    f.render_widget(Clear, area);
    f.render_widget(name_paragraph, split_popup[0]);
    f.render_widget(description_paragraph, split_popup[1]);
    f.render_widget(help_paragraph, split_popup[2]);
    f.render_widget(final_paragraph, split_popup[3]);
    f.render_widget(block, area);
}

/// Draw the delete confirm box in the centre of the screen
/// take in container id and container name here?
pub fn delete_confirm<B: Backend>(
    f: &mut Frame<'_, B>,
    gui_state: &Arc<Mutex<GuiState>>,
    name: &str,
) {
    let block = Block::default()
        .title(" Confirm Delete ")
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(Color::White).fg(Color::Black))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL);

    let confirm = Line::from(vec![
        Span::from("Are you sure you want to delete container: "),
        Span::styled(
            name,
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
    ]);

    let yes_text = " (Y)es ";
    let no_text = " (N)o ";

    // Find the maximum line width & height, and add some padding
    let max_line_width = u16::try_from(confirm.width()).unwrap_or(64) + 12;
    let lines = 8;

    let confirm_para = Paragraph::new(confirm).alignment(Alignment::Center);

    let button_block = || {
        Block::default()
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL)
    };

    let yes_para = Paragraph::new(yes_text)
        .alignment(Alignment::Center)
        .block(button_block());
    // Need to add some padding for the borders
    let yes_chars = u16::try_from(yes_text.chars().count() + 2).unwrap_or(9);

    let no_para = Paragraph::new(no_text)
        .alignment(Alignment::Center)
        .block(button_block());
    // Need to add some padding for the borders
    let no_chars = u16::try_from(no_text.chars().count() + 2).unwrap_or(8);

    let area = popup(
        lines,
        max_line_width.into(),
        f.size(),
        BoxLocation::MiddleCentre,
    );

    let split_popup = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Min(2),
                Constraint::Max(1),
                Constraint::Max(1),
                Constraint::Max(3),
                Constraint::Min(1),
            ]
            .as_ref(),
        )
        .split(area);

    // Should maybe have a differenet button_space IF the f.width() is within 2 chars of no_chars + yes_chars?
    let button_spacing = (max_line_width - no_chars - yes_chars) / 3;

    let button_spacing = if (button_spacing + max_line_width) > f.size().width {
        1
    } else {
        button_spacing
    };
    let split_buttons = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Max(button_spacing),
                Constraint::Min(no_chars),
                Constraint::Max(button_spacing),
                Constraint::Min(yes_chars),
                Constraint::Max(button_spacing),
            ]
            .as_ref(),
        )
        .split(split_popup[3]);

    let no_area = split_buttons[1];
    let yes_area = split_buttons[3];

    // Insert button areas into region map, so can interact with them on click
    gui_state
        .lock()
        .update_region_map(Region::Delete(DeleteButton::No), no_area);

    gui_state
        .lock()
        .update_region_map(Region::Delete(DeleteButton::Yes), yes_area);

    f.render_widget(Clear, area);
    f.render_widget(block, area);
    f.render_widget(confirm_para, split_popup[1]);
    f.render_widget(no_para, no_area);
    f.render_widget(yes_para, yes_area);
}

/// Draw an error popup over whole screen
pub fn error<B: Backend>(f: &mut Frame<'_, B>, error: AppError, seconds: Option<u8>) {
    let block = Block::default()
        .title(" Error ")
        .border_type(BorderType::Rounded)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL);

    let to_push = match error {
        AppError::DockerConnect => {
            format!(
                "\n\n {}::v{} closing in {:02} seconds",
                NAME,
                VERSION,
                seconds.unwrap_or(5)
            )
        }
        _ => String::from("\n\n ( c ) to clear error\n ( q ) to quit oxker"),
    };

    let mut text = format!("\n{error}");

    text.push_str(to_push.as_str());

    // Find the maximum line width & height
    let mut max_line_width = max_line_width(&text);
    let mut lines = text.lines().count();

    // Add some horizontal & vertical margins
    max_line_width += 8;
    lines += 3;

    let paragraph = Paragraph::new(text)
        .style(Style::default().bg(Color::Red).fg(Color::White))
        .block(block)
        .alignment(Alignment::Center);

    let area = popup(lines, max_line_width, f.size(), BoxLocation::MiddleCentre);
    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}

/// Draw info box in one of the 9 BoxLocations
pub fn info<B: Backend>(f: &mut Frame<'_, B>, text: String) {
    let block = Block::default()
        .title("")
        .title_alignment(Alignment::Center)
        .borders(Borders::NONE);

    let mut max_line_width = max_line_width(&text);
    let mut lines = text.lines().count();

    // Add some horizontal & vertical margins
    max_line_width += 8;
    lines += 2;

    let paragraph = Paragraph::new(text)
        .style(Style::default().bg(Color::Blue).fg(Color::White))
        .block(block)
        .alignment(Alignment::Center);

    let area = popup(lines, max_line_width, f.size(), BoxLocation::BottomRight);
    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}

/// draw a box in the one of the BoxLocations, based on max line width + number of lines
fn popup(text_lines: usize, text_width: usize, r: Rect, box_location: BoxLocation) -> Rect {
    // Make sure blank_space can't be an negative, as will crash
    let calc = |x: u16, y: usize| usize::from(x).saturating_sub(y).saturating_div(2);

    let blank_vertical = calc(r.height, text_lines);
    let blank_horizontal = calc(r.width, text_width);

    let (h_constraints, v_constraints) = box_location.get_constraints(
        blank_horizontal.try_into().unwrap_or_default(),
        blank_vertical.try_into().unwrap_or_default(),
        text_lines.try_into().unwrap_or_default(),
        text_width.try_into().unwrap_or_default(),
    );

    let indexes = box_location.get_indexes();

    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(v_constraints)
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(h_constraints)
        .split(popup_layout[indexes.0])[indexes.1]
}

// Draw nothing, as in a blank screen
// pub fn nothing<B: Backend>(f: &mut Frame<'_, B>) {
//     let whole_layout = Layout::default()
//         .direction(Direction::Vertical)
//         .constraints([Constraint::Min(100)].as_ref())
//         .split(f.size());

//     let block = Block::default()
//         .borders(Borders::NONE);
//     f.render_widget(block, whole_layout[0]);
// }
