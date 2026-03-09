//! Display module - handles terminal UI

use std::{io, time::Duration};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Cell as TuiCell, Paragraph, Row, Table, Tabs},
    Frame, Terminal,
};

use crate::sheet::{Sheet, Adjust};
use crate::token::Token;

// UI state
struct DisplayState {
    always_redraw: bool,
    status_message: String,
    input_mode: InputMode,
    input_buffer: String,
    cursor_position: usize,
}

// Input modes
enum InputMode {
    Normal,
    Editing,
    Command,
}

impl Default for DisplayState {
    fn default() -> Self {
        Self {
            always_redraw: false,
            status_message: String::from("Welcome to Teapot"),
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            cursor_position: 0,
        }
    }
}

/// Initialize the display
pub fn display_init(sheet: &Sheet, always_redraw: bool) {
    // Store the always_redraw flag for later use
    // This will be used in the actual implementation
}

/// Main display loop
pub fn display_main(sheet: &mut Sheet) {
    // Setup terminal
    enable_raw_mode().expect("Failed to enable raw mode");
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .expect("Failed to enter alternate screen");
    
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");
    
    // Create app state
    let mut state = DisplayState::default();
    
    // Run the main loop
    let res = run_app(&mut terminal, sheet, &mut state);
    
    // Restore terminal
    disable_raw_mode().expect("Failed to disable raw mode");
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    ).expect("Failed to leave alternate screen");
    terminal.show_cursor().expect("Failed to show cursor");
    
    if let Err(err) = res {
        println!("Error: {:?}", err);
    }
}

/// Clean up and end the display
pub fn display_end() {
    // This is now handled in display_main
}

/// Redraw a single cell
pub fn redraw_cell(sheet: &Sheet, x: usize, y: usize, z: usize) {
    // This will be handled by the main rendering loop
}

/// Redraw the entire sheet
pub fn redraw_sheet(sheet: &Sheet) {
    // This will be handled by the main rendering loop
}

// Main application loop
fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    sheet: &mut Sheet,
    state: &mut DisplayState,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, sheet, state))?;

        if let Event::Key(key) = event::read()? {
            match state.input_mode {
                InputMode::Normal => {
                    match key.code {
                        KeyCode::Char('q') => {
                            if key.modifiers.contains(KeyModifiers::CONTROL) {
                                return Ok(());
                            }
                        }
                        KeyCode::Char('e') => {
                            state.input_mode = InputMode::Editing;
                            state.input_buffer.clear();
                        }
                        KeyCode::Char(':') => {
                            state.input_mode = InputMode::Command;
                            state.input_buffer.clear();
                            state.input_buffer.push(':');
                            state.cursor_position = 1;
                        }
                        KeyCode::Up => {
                            if sheet.cur_y > 0 {
                                sheet.cur_y -= 1;
                            }
                        }
                        KeyCode::Down => {
                            sheet.cur_y += 1;
                        }
                        KeyCode::Left => {
                            if sheet.cur_x > 0 {
                                sheet.cur_x -= 1;
                            }
                        }
                        KeyCode::Right => {
                            sheet.cur_x += 1;
                        }
                        KeyCode::Tab => {
                            if sheet.cur_z + 1 < sheet.dim_z {
                                sheet.cur_z += 1;
                            } else {
                                sheet.cur_z = 0;
                            }
                        }
                        _ => {}
                    }
                }
                InputMode::Editing => {
                    match key.code {
                        KeyCode::Enter => {
                            // Process cell edit
                            let cell = sheet.get_or_create_cell(sheet.cur_x, sheet.cur_y, sheet.cur_z);
                            // TODO: Parse input_buffer into tokens
                            state.status_message = format!("Cell edited: {}", state.input_buffer);
                            state.input_mode = InputMode::Normal;
                        }
                        KeyCode::Esc => {
                            state.input_mode = InputMode::Normal;
                        }
                        KeyCode::Char(c) => {
                            state.input_buffer.push(c);
                        }
                        KeyCode::Backspace => {
                            state.input_buffer.pop();
                        }
                        _ => {}
                    }
                }
                InputMode::Command => {
                    match key.code {
                        KeyCode::Enter => {
                            // Process command
                            process_command(sheet, state);
                            state.input_mode = InputMode::Normal;
                        }
                        KeyCode::Esc => {
                            state.input_mode = InputMode::Normal;
                        }
                        KeyCode::Char(c) => {
                            state.input_buffer.push(c);
                            state.cursor_position += 1;
                        }
                        KeyCode::Backspace => {
                            if state.cursor_position > 1 {
                                state.input_buffer.pop();
                                state.cursor_position -= 1;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

// Process command input
fn process_command(sheet: &mut Sheet, state: &mut DisplayState) {
    let cmd = state.input_buffer.trim_start_matches(':').trim();
    
    if cmd.starts_with("q") || cmd.starts_with("quit") {
        state.status_message = String::from("Use Ctrl+Q to quit");
    } else if cmd.starts_with("w") || cmd.starts_with("write") {
        state.status_message = String::from("Save functionality not implemented yet");
    } else if cmd.starts_with("goto") {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.len() > 1 {
            let coords: Vec<&str> = parts[1].split(',').collect();
            if coords.len() >= 2 {
                if let (Ok(x), Ok(y)) = (coords[0].parse::<usize>(), coords[1].parse::<usize>()) {
                    sheet.cur_x = x;
                    sheet.cur_y = y;
                    if coords.len() > 2 {
                        if let Ok(z) = coords[2].parse::<usize>() {
                            sheet.cur_z = z;
                        }
                    }
                    state.status_message = format!("Moved to ({}, {}, {})", sheet.cur_x, sheet.cur_y, sheet.cur_z);
                } else {
                    state.status_message = String::from("Invalid coordinates");
                }
            } else {
                state.status_message = String::from("Expected format: goto x,y[,z]");
            }
        } else {
            state.status_message = String::from("Expected format: goto x,y[,z]");
        }
    } else {
        state.status_message = format!("Unknown command: {}", cmd);
    }
}

// UI rendering function
fn ui(f: &mut Frame, sheet: &Sheet, state: &DisplayState) {
    // Create layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),  // Tabs
            Constraint::Min(0),     // Spreadsheet
            Constraint::Length(1),  // Status
            Constraint::Length(1),  // Input
        ])
        .split(f.size());
    
    // Render tabs for sheets
    let tab_titles = vec![
        Span::styled(
            format!("Sheet {}", sheet.cur_z + 1),
            Style::default().fg(Color::Green)
        ),
    ];
    
    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::BOTTOM))
        .highlight_style(Style::default().fg(Color::Yellow))
        .select(sheet.cur_z);
    
    f.render_widget(tabs, chunks[0]);
    
    // Render spreadsheet
    render_sheet(f, sheet, chunks[1]);
    
    // Render status bar
    let status = Paragraph::new(state.status_message.as_str())
        .style(Style::default().fg(Color::White).bg(Color::Blue));
    f.render_widget(status, chunks[2]);
    
    // Render input bar
    match state.input_mode {
        InputMode::Normal => {
            let input = Paragraph::new("Press 'e' to edit cell, ':' for command mode")
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(input, chunks[3]);
        }
        InputMode::Editing => {
            let input = Paragraph::new(state.input_buffer.as_str())
                .style(Style::default().fg(Color::Yellow));
            f.render_widget(input, chunks[3]);
            f.set_cursor(
                chunks[3].x + state.input_buffer.len() as u16,
                chunks[3].y,
            );
        }
        InputMode::Command => {
            let input = Paragraph::new(state.input_buffer.as_str())
                .style(Style::default().fg(Color::Yellow));
            f.render_widget(input, chunks[3]);
            f.set_cursor(
                chunks[3].x + state.cursor_position as u16,
                chunks[3].y,
            );
        }
    }
}

// Render the spreadsheet
fn render_sheet(f: &mut Frame, sheet: &Sheet, area: Rect) {
    // Calculate visible range
    let visible_cols = (area.width as usize).saturating_sub(4).min(sheet.dim_x);
    let visible_rows = (area.height as usize).saturating_sub(2).min(sheet.dim_y);
    
    // Create column headers
    let mut header_cells = vec![TuiCell::from("#")];
    for x in 0..visible_cols {
        header_cells.push(TuiCell::from(format!("{}", x + 1)));
    }
    let header = Row::new(header_cells)
        .style(Style::default().fg(Color::Yellow))
        .height(1);
    
    // Create rows
    let mut rows = vec![header];
    for y in 0..visible_rows {
        let mut cells = vec![TuiCell::from(format!("{}", y + 1))
            .style(Style::default().fg(Color::Yellow))];
        
        for x in 0..visible_cols {
            let cell_content = if let Some(cell) = sheet.get_cell(x, y, sheet.cur_z) {
                match &cell.value {
                    Token::Empty => String::from(""),
                    Token::Integer(i) => i.to_string(),
                    Token::Float(f) => f.to_string(),
                    Token::String(s) => s.clone(),
                    Token::Error(e) => format!("ERROR: {}", e),
                    Token::Location(loc) => format!("@({},{},{})", loc[0], loc[1], loc[2]),
                    Token::Identifier(id) => id.clone(),
                    Token::LabelIdentifier(id) => id.clone(),
                    Token::Operator(op) => op.to_string(),
                }
            } else {
                String::from("")
            };
            
            let style = if x == sheet.cur_x && y == sheet.cur_y {
                Style::default().fg(Color::Black).bg(Color::White)
            } else {
                Style::default()
            };
            
            cells.push(TuiCell::from(cell_content).style(style));
        }
        
        rows.push(Row::new(cells).height(1));
    }
    
    // Create table
    let widths = [
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Min(5),
        Constraint::Min(5),
        Constraint::Min(5),
        Constraint::Min(5),
    ];
    
    let table = Table::new(rows, widths)
        .block(Block::default().borders(Borders::ALL).title("Teapot"))
        .column_spacing(1);
    
    f.render_widget(table, area);
}
