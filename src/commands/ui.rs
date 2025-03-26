use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, MouseEvent, MouseEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Table, Row, Tabs, Paragraph},
    layout::{Constraint, Direction, Layout, Position},
    style::{Style, Modifier, Stylize},
};
use std::io::stdout;
use serde_json::Value;
use strum_macros::Display;

use crate::client::Client;

// Constants for icons (with added space after each icon)
const WINDOWS_ICON: &str = "\u{f17a} ";    // Windows icon
const LINUX_ICON: &str = "\u{f17c} ";      // Linux icon
const VIRTUAL_ICON: &str = "\u{f0c2} ";    // Virtual icon (cloud)
const UNKNOWN_ICON: &str = "\u{f128} ";    // Question mark icon
const WINDOWS_FALLBACK: &str = "[W] ";
const LINUX_FALLBACK: &str = "[L] ";
const VIRTUAL_FALLBACK: &str = "[V] ";
const UNKNOWN_FALLBACK: &str = "[?] ";

// Add this near the top of the file with other constants
const TAB_WIDTH: u16 = 15;

// Add tab state enum
#[derive(Default, Clone, Copy, Display)]
enum SelectedTab {
    #[default]
    VolumeShow,
    Scans,
    Browse,
}

impl SelectedTab {
    fn next(self) -> Self {
        match self {
            Self::VolumeShow => Self::Scans,
            Self::Scans => Self::Browse,
            Self::Browse => Self::Browse, // Stay on last tab
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::VolumeShow => Self::VolumeShow, // Stay on first tab
            Self::Scans => Self::VolumeShow,
            Self::Browse => Self::Scans,
        }
    }

    fn all() -> Vec<Self> {
        vec![Self::VolumeShow, Self::Scans, Self::Browse]
    }

    fn to_index(&self) -> usize {
        match self {
            Self::VolumeShow => 0,
            Self::Scans => 1,
            Self::Browse => 2,
        }
    }

    fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::VolumeShow),
            1 => Some(Self::Scans),
            2 => Some(Self::Browse),
            _ => None,
        }
    }

    fn title(&self) -> String {
        let base_title = match self {
            Self::VolumeShow => "Volume Show [1]",
            Self::Scans => "Scans [2]     ",  // padding with spaces
            Self::Browse => "Browse [3]    ",  // padding with spaces
        };
        format!("{:width$}", base_title, width = TAB_WIDTH as usize)  // use constant
    }

    fn from_key(key: char) -> Option<Self> {
        match key {
            '1' => Some(Self::VolumeShow),
            '2' => Some(Self::Scans),
            '3' => Some(Self::Browse),
            _ => None,
        }
    }
}

struct TableState {
    selected: Option<usize>,
    items: Vec<Value>,
    use_unicode: bool,
    selected_tab: SelectedTab,
}

impl TableState {
    fn new(mut items: Vec<Value>) -> Self {
        // Sort volumes by name
        items.sort_by(|a, b| {
            let name_a = a["vol"].as_str().unwrap_or("");
            let name_b = b["vol"].as_str().unwrap_or("");
            name_a.cmp(name_b)
        });
        
        // Test if terminal can display unicode icons
        let use_unicode = String::from(WINDOWS_ICON).chars().all(|c| !c.is_control()) 
            && String::from(LINUX_ICON).chars().all(|c| !c.is_control())
            && String::from(VIRTUAL_ICON).chars().all(|c| !c.is_control())
            && String::from(UNKNOWN_ICON).chars().all(|c| !c.is_control());
        
        Self {
            selected: if items.is_empty() { None } else { Some(0) },
            items,
            use_unicode,
            selected_tab: SelectedTab::default(),
        }
    }

    fn get_os_icon_with_style(&self, vol_type: &str) -> (String, Style) {
        let icon = if self.use_unicode {
            match vol_type.to_lowercase().as_str() {
                "windows" => WINDOWS_ICON,
                "linux" => LINUX_ICON,
                "virtual" => VIRTUAL_ICON,
                _ => UNKNOWN_ICON,
            }
        } else {
            match vol_type.to_lowercase().as_str() {
                "windows" => WINDOWS_FALLBACK,
                "linux" => LINUX_FALLBACK,
                "virtual" => VIRTUAL_FALLBACK,
                _ => UNKNOWN_FALLBACK,
            }
        };
        
        (icon.to_string(), Style::default())
    }

    fn next(&mut self) {
        if self.items.is_empty() {
            self.selected = None;
        } else {
            self.selected = Some(match self.selected {
                Some(i) => (i + 1) % self.items.len(),
                None => 0,
            });
        }
    }

    fn previous(&mut self) {
        if self.items.is_empty() {
            self.selected = None;
        } else {
            self.selected = Some(match self.selected {
                Some(i) => {
                    if i == 0 {
                        self.items.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            });
        }
    }
}

pub async fn handle_ui_command(client: &Client) -> Result<()> {
    // Enable mouse capture when initializing terminal
    stdout().execute(crossterm::event::EnableMouseCapture)?;
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    // Get initial volumes data
    let volumes = client.get_volumes().await?;
    let volumes_array = volumes.as_array().unwrap_or(&vec![]).clone();
    let mut table_state = TableState::new(volumes_array);
    let mut selection_state = ratatui::widgets::TableState::default();
    selection_state.select(table_state.selected);
    
    // First, let's store the areas in the main loop scope
    let mut volumes_area = Rect::default();
    let mut tabs_area = Rect::default();

    // Run the UI loop
    loop {
        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(20),
                    Constraint::Percentage(80),
                ])
                .split(frame.size());
            
            // Store the areas for use in mouse handling
            volumes_area = chunks[0];
            tabs_area = chunks[1];
            
            // Create table rows with styled OS icons
            let rows: Vec<Row> = table_state.items.iter()
                .filter_map(|volume| {
                    let name = volume["vol"].as_str()?;
                    let vol_type = volume["type"].as_str().unwrap_or("");
                    let (icon, style) = table_state.get_os_icon_with_style(vol_type);
                    
                    // Create a styled row with the icon and name
                    Some(Row::new(vec![
                        format!("{}{}", icon, name)
                    ]).style(style))
                })
                .collect();

            let table = Table::new(
                rows,
                vec![Constraint::Percentage(100)],
            )
            .block(Block::default().title("Volumes").borders(Borders::ALL))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

            frame.render_stateful_widget(table, volumes_area, &mut selection_state);

            // Right panel with improved tabs
            let right_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),  // Changed from 3 to 1 for tab header
                    Constraint::Min(0),
                ])
                .split(chunks[1]);

            // Create tabs with improved styling
            let titles = SelectedTab::all()
                .iter()
                .map(|t| t.title())  // This will now use TAB_WIDTH internally
                .collect::<Vec<_>>();
            
            let tabs = Tabs::new(titles)
                .select(table_state.selected_tab.to_index())
                .highlight_style(Style::default().bold())
                .divider(symbols::line::VERTICAL);

            frame.render_widget(tabs, right_chunks[0]);

            // Render content with connected border
            match table_state.selected_tab {
                SelectedTab::VolumeShow => {
                    let details_text = match table_state.selected {
                        Some(index) => {
                            if let Some(volume) = table_state.items.get(index) {
                                serde_json::to_string_pretty(volume).unwrap_or_else(|_| "Error formatting JSON".to_string())
                            } else {
                                "No volume selected".to_string()
                            }
                        }
                        None => "No volume selected".to_string(),
                    };

                    let details = Paragraph::new(details_text)
                        .block(Block::default()
                            .borders(Borders::ALL)
                            .border_set(symbols::border::PLAIN)
                            .border_style(Style::default()))
                        .wrap(ratatui::widgets::Wrap { trim: true });

                    frame.render_widget(details, right_chunks[1]);
                }
                SelectedTab::Scans => {
                    let content = Paragraph::new("Scans tab content coming soon...")
                        .block(Block::default()
                            .borders(Borders::ALL)
                            .border_set(symbols::border::PLAIN)
                            .border_style(Style::default()));
                    frame.render_widget(content, right_chunks[1]);
                }
                SelectedTab::Browse => {
                    let content = Paragraph::new("Browse tab content coming soon...")
                        .block(Block::default()
                            .borders(Borders::ALL)
                            .border_set(symbols::border::PLAIN)
                            .border_style(Style::default()));
                    frame.render_widget(content, right_chunks[1]);
                }
            }
        })?;

        // Handle input with new keyboard shortcuts
        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char(c) => {
                            if let Some(tab) = SelectedTab::from_key(c) {
                                table_state.selected_tab = tab;
                            } else {
                                match c {
                                    'j' | 'k' => {
                                        if c == 'j' {
                                            table_state.next();
                                        } else {
                                            table_state.previous();
                                        }
                                        selection_state.select(table_state.selected);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        KeyCode::Down => {
                            table_state.next();
                            selection_state.select(table_state.selected);
                        }
                        KeyCode::Up => {
                            table_state.previous();
                            selection_state.select(table_state.selected);
                        }
                        KeyCode::Right => {
                            table_state.selected_tab = table_state.selected_tab.next();
                        }
                        KeyCode::Left => {
                            table_state.selected_tab = table_state.selected_tab.previous();
                        }
                        _ => {}
                    }
                }
                Event::Mouse(MouseEvent { kind, row, column, .. }) => {
                    if let MouseEventKind::Down(_) = kind {
                        // Create a Position from the mouse coordinates
                        let mouse_point = Position { x: column, y: row };

                        // Handle volume list clicks
                        if volumes_area.contains(mouse_point) {
                            // Convert to relative position within the volumes area
                            let relative_row = row.saturating_sub(volumes_area.y + 1); // +1 to account for border
                            if relative_row < table_state.items.len() as u16 {
                                table_state.selected = Some(relative_row as usize);
                                selection_state.select(Some(relative_row as usize));
                            }
                        }
                        // Handle tab clicks
                        else if tabs_area.contains(mouse_point) {
                            // Only handle clicks in the tab header row (right_chunks[0])
                            if row == tabs_area.y {  // First row of tabs area
                                // Convert to relative position within the tabs area
                                let relative_x = column.saturating_sub(tabs_area.x);
                                let tab_index = relative_x / (TAB_WIDTH + 2);
                                if tab_index < 3 {  // We have 3 tabs
                                    if let Some(tab) = SelectedTab::from_index(tab_index as usize) {
                                        table_state.selected_tab = tab;
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Disable mouse capture when cleaning up
    stdout().execute(crossterm::event::DisableMouseCapture)?;
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    Ok(())
} 