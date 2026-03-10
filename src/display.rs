use std::io;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Cell as TuiCell, Paragraph, Row, Table, Tabs},
    Frame, Terminal,
};

use crate::scanner;
use crate::sheet::Sheet;
use crate::token::Token;

struct DisplayState {
    status_message: String,
    input_mode: InputMode,
    input_buffer: String,
    cursor_position: usize,
    terminal_area: Rect,
}

enum InputMode {
    Normal,
    Editing,
    Command,
    Help,
}

impl Default for DisplayState {
    fn default() -> Self {
        Self {
            status_message: String::from("Welcome to Teapot"),
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            cursor_position: 0,
            terminal_area: Rect::default(),
        }
    }
}

/// Main display loop
pub fn display_main(sheet: &mut Sheet) {
    enable_raw_mode().expect("Failed to enable raw mode");
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .expect("Failed to enter alternate screen");

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let mut state = DisplayState::default();

    let res = run_app(&mut terminal, sheet, &mut state);

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

/// Number of data rows visible in the terminal area
fn visible_rows(area: Rect) -> usize {
    // tabs (1) + table borders (2) + header row (1) + status (1) + input (1) = 6
    (area.height as usize).saturating_sub(6).max(1)
}

/// Number of data columns visible in the terminal area
fn visible_cols(sheet: &Sheet, area: Rect) -> usize {
    let row_num_width = 4u16;
    let mut used = row_num_width;
    let mut count = 0;
    for x in sheet.off_x..sheet.off_x + 100 {
        let w = sheet.column_width(x, sheet.cur_z) as u16;
        if used + w + 1 > area.width {
            break;
        }
        used += w + 1;
        count += 1;
    }
    count.max(1)
}

/// Adjust viewport offset so the cursor stays visible on screen
fn adjust_viewport(sheet: &mut Sheet, area: Rect) {
    let row_num_width = 4u16;
    let visible_rows = visible_rows(area);

    // Vertical scrolling
    if sheet.cur_y < sheet.off_y {
        sheet.off_y = sheet.cur_y;
    } else if visible_rows > 0 && sheet.cur_y >= sheet.off_y + visible_rows {
        sheet.off_y = sheet.cur_y - visible_rows + 1;
    }

    // Horizontal scrolling
    if sheet.cur_x < sheet.off_x {
        sheet.off_x = sheet.cur_x;
    } else {
        // Check if cur_x is visible from current off_x
        let mut used = row_num_width;
        let mut last_visible_x = sheet.off_x;
        for x in sheet.off_x..=sheet.cur_x {
            let w = sheet.column_width(x, sheet.cur_z) as u16;
            if used + w + 1 > area.width {
                break;
            }
            last_visible_x = x;
            used += w + 1;
        }
        if sheet.cur_x > last_visible_x {
            // Scroll right: find the leftmost off_x that makes cur_x visible
            let mut used = row_num_width;
            let mut new_off = sheet.cur_x;
            for x in (0..=sheet.cur_x).rev() {
                let w = sheet.column_width(x, sheet.cur_z) as u16;
                if used + w + 1 > area.width {
                    break;
                }
                new_off = x;
                used += w + 1;
            }
            sheet.off_x = new_off;
        }
    }
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    sheet: &mut Sheet,
    state: &mut DisplayState,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| {
            state.terminal_area = f.area();
            ui(f, sheet, state);
        })?;

        if let Event::Key(key) = event::read()? {
            match state.input_mode {
                InputMode::Normal => {
                    match key.code {
                        KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            return Ok(());
                        }
                        KeyCode::Char('e') | KeyCode::Enter => {
                            state.input_mode = InputMode::Editing;
                            state.input_buffer.clear();
                            // Pre-fill with current cell formula
                            if let Some(cell) = sheet.get_cell(sheet.cur_x, sheet.cur_y, sheet.cur_z) {
                                if let Some(ref contents) = cell.contents {
                                    state.input_buffer = scanner::print_tokens(contents, true, cell.scientific, cell.precision);
                                }
                            }
                            state.cursor_position = state.input_buffer.len();
                        }
                        KeyCode::Char(':') => {
                            state.input_mode = InputMode::Command;
                            state.input_buffer.clear();
                            state.input_buffer.push(':');
                            state.cursor_position = 1;
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            if sheet.cur_y > 0 { sheet.cur_y -= 1; }
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            sheet.cur_y += 1;
                        }
                        KeyCode::Char('h') | KeyCode::Left => {
                            if sheet.cur_x > 0 { sheet.cur_x -= 1; }
                        }
                        KeyCode::Char('l') | KeyCode::Right => {
                            sheet.cur_x += 1;
                        }
                        KeyCode::Char('K') => {
                            let page = visible_rows(state.terminal_area);
                            sheet.cur_y = sheet.cur_y.saturating_sub(page);
                        }
                        KeyCode::Char('J') => {
                            let page = visible_rows(state.terminal_area);
                            sheet.cur_y += page;
                        }
                        KeyCode::Char('H') => {
                            let page = visible_cols(sheet, state.terminal_area);
                            sheet.cur_x = sheet.cur_x.saturating_sub(page);
                        }
                        KeyCode::Char('L') => {
                            let page = visible_cols(sheet, state.terminal_area);
                            sheet.cur_x += page;
                        }
                        KeyCode::Tab => {
                            if sheet.cur_z + 1 < sheet.dim_z {
                                sheet.cur_z += 1;
                            } else {
                                sheet.cur_z = 0;
                            }
                        }
                        KeyCode::Char('?') => {
                            state.input_mode = InputMode::Help;
                        }
                        KeyCode::Delete => {
                            // Clear current cell
                            let (x, y, z) = (sheet.cur_x, sheet.cur_y, sheet.cur_z);
                            if let Some(cell) = sheet.get_cell_mut(x, y, z) {
                                cell.contents = None;
                                cell.value = Token::Empty;
                                sheet.changed = true;
                            }
                            state.status_message = format!("Cell ({},{},{}) cleared", x, y, z);
                        }
                        _ => {}
                    }
                    adjust_viewport(sheet, state.terminal_area);
                }
                InputMode::Editing => {
                    match key.code {
                        KeyCode::Enter => {
                            let input = state.input_buffer.trim().to_string();
                            let (x, y, z) = (sheet.cur_x, sheet.cur_y, sheet.cur_z);
                            if input.is_empty() {
                                // Clear the cell
                                if let Some(cell) = sheet.get_cell_mut(x, y, z) {
                                    cell.contents = None;
                                    cell.value = Token::Empty;
                                }
                                state.status_message = format!("Cell ({},{},{}) cleared", x, y, z);
                            } else {
                                match scanner::scan(&input) {
                                    Ok(tokens) => {
                                        sheet.putcont(x, y, z, tokens);
                                        sheet.update();
                                        let val = sheet.get_cell(x, y, z)
                                            .map(|c| format!("{}", c.value))
                                            .unwrap_or_default();
                                        state.status_message = format!("({},{},{}) = {}", x, y, z, val);
                                    }
                                    Err(e) => {
                                        state.status_message = format!("Parse error: {}", e);
                                    }
                                }
                            }
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
                            if !state.input_buffer.is_empty() {
                                state.input_buffer.pop();
                                state.cursor_position = state.cursor_position.saturating_sub(1);
                            }
                        }
                        _ => {}
                    }
                }
                InputMode::Command => {
                    match key.code {
                        KeyCode::Enter => {
                            process_command(sheet, state);
                            if !matches!(state.input_mode, InputMode::Help) {
                                state.input_mode = InputMode::Normal;
                            }
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
                InputMode::Help => {
                    // Any key exits help
                    state.input_mode = InputMode::Normal;
                }
            }
        }
    }
}

fn process_command(sheet: &mut Sheet, state: &mut DisplayState) {
    let cmd = state.input_buffer.trim_start_matches(':').trim();
    let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
    let command = parts[0];
    let arg = parts.get(1).map(|s| s.trim()).unwrap_or("");

    match command {
        "q" | "quit" => {
            state.status_message = String::from("Use Ctrl+Q to quit");
        }
        "w" | "write" => {
            let filename = if arg.is_empty() {
                sheet.name.clone().unwrap_or_else(|| "sheet.tpa".to_string())
            } else {
                arg.to_string()
            };
            match crate::fileio::save_port(sheet, &filename) {
                Ok(count) => {
                    sheet.name = Some(filename.clone());
                    sheet.changed = false;
                    state.status_message = format!("Saved {} cells to {}", count, filename);
                }
                Err(e) => {
                    state.status_message = format!("Save failed: {}", e);
                }
            }
        }
        "goto" => {
            if arg.is_empty() {
                state.status_message = String::from("Usage: goto x,y[,z]");
            } else {
                let coords: Vec<&str> = arg.split(',').collect();
                if coords.len() >= 2 {
                    if let (Ok(x), Ok(y)) = (coords[0].trim().parse::<usize>(), coords[1].trim().parse::<usize>()) {
                        sheet.cur_x = x;
                        sheet.cur_y = y;
                        if coords.len() > 2 {
                            if let Ok(z) = coords[2].trim().parse::<usize>() {
                                sheet.cur_z = z;
                            }
                        }
                        state.status_message = format!("Moved to ({},{},{})", sheet.cur_x, sheet.cur_y, sheet.cur_z);
                    } else {
                        state.status_message = String::from("Invalid coordinates");
                    }
                } else {
                    state.status_message = String::from("Usage: goto x,y[,z]");
                }
            }
        }
        "help" => {
            state.input_mode = InputMode::Help;
        }
        _ => {
            state.status_message = format!("Unknown command: {}", cmd);
        }
    }
}

fn ui(f: &mut Frame, sheet: &Sheet, state: &DisplayState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints([
            Constraint::Length(1),  // Tabs
            Constraint::Min(0),    // Spreadsheet
            Constraint::Length(1), // Status
            Constraint::Length(1), // Input/formula bar
        ])
        .split(f.area());

    // Tabs
    let tab_titles: Vec<Span> = (0..sheet.dim_z.max(1))
        .map(|i| {
            let style = if i == sheet.cur_z {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            Span::styled(format!(" Sheet {} ", i + 1), style)
        })
        .collect();
    let tabs = Tabs::new(tab_titles).select(sheet.cur_z);
    f.render_widget(tabs, chunks[0]);

    // Spreadsheet or Help
    if matches!(state.input_mode, InputMode::Help) {
        render_help(f, chunks[1]);
    } else {
        render_sheet(f, sheet, chunks[1]);
    }

    // Status bar: show cell info
    let cell_info = if let Some(cell) = sheet.get_cell(sheet.cur_x, sheet.cur_y, sheet.cur_z) {
        let formula = cell.contents.as_ref()
            .map(|c| scanner::print_tokens(c, true, cell.scientific, cell.precision))
            .unwrap_or_default();
        format!("({},{},{}) {} | {}", sheet.cur_x, sheet.cur_y, sheet.cur_z, formula, state.status_message)
    } else {
        format!("({},{},{}) | {}", sheet.cur_x, sheet.cur_y, sheet.cur_z, state.status_message)
    };
    let status = Paragraph::new(cell_info)
        .style(Style::default().fg(Color::White).bg(Color::Blue));
    f.render_widget(status, chunks[2]);

    // Input bar
    match state.input_mode {
        InputMode::Normal => {
            let help = Paragraph::new(" hjkl: move | HJKL: page | e/Enter: edit | :: command | Ctrl+Q: quit")
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(help, chunks[3]);
        }
        InputMode::Editing => {
            let input = Paragraph::new(format!("Edit: {}", state.input_buffer))
                .style(Style::default().fg(Color::Yellow));
            f.render_widget(input, chunks[3]);
            f.set_cursor_position((
                chunks[3].x + 6 + state.cursor_position as u16,
                chunks[3].y,
            ));
        }
        InputMode::Command => {
            let input = Paragraph::new(state.input_buffer.as_str())
                .style(Style::default().fg(Color::Yellow));
            f.render_widget(input, chunks[3]);
            f.set_cursor_position((
                chunks[3].x + state.cursor_position as u16,
                chunks[3].y,
            ));
        }
        InputMode::Help => {
            let help = Paragraph::new(" Press any key to return")
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(help, chunks[3]);
        }
    }
}

fn render_help(f: &mut Frame, area: Rect) {
    let help_text = "\
  Teapot — Table Editor And Planner

  Navigation
    h / Left      Move left           H   Page left
    j / Down      Move down           J   Page down
    k / Up        Move up             K   Page up
    l / Right     Move right          L   Page right
    Tab           Next sheet

  Editing
    e / Enter     Edit cell (enter formula or value)
    Delete        Clear current cell
    Esc           Cancel editing

  Commands (press : to enter command mode)
    :w [file]     Save (.tpa format)
    :goto x,y[,z] Move to cell
    :help         Show this help
    :q            Quit hint (use Ctrl+Q)

  Formulas
    Numbers       42, 3.14
    Strings       \"hello\"
    Arithmetic    1+2, 3*4, 2^10
    Cell ref      @(x,y,z)
    Functions     sum, min, max, abs, sin, cos, len, substr, ...

  Press any key to return";

    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .style(Style::default().fg(Color::White));
    f.render_widget(help, area);
}

fn render_sheet(f: &mut Frame, sheet: &Sheet, area: Rect) {
    if area.height < 3 || area.width < 6 {
        return;
    }

    let z = sheet.cur_z;

    // Calculate visible columns based on actual column widths
    let row_num_width = 4u16;
    let mut col_widths: Vec<(usize, u16)> = Vec::new(); // (col_index, width)
    let mut used_width = row_num_width;

    let start_col = sheet.off_x;
    for x in start_col..sheet.dim_x {
        let w = sheet.column_width(x, z) as u16;
        if used_width + w + 1 > area.width {
            break;
        }
        col_widths.push((x, w));
        used_width += w + 1;
    }

    let start_row = sheet.off_y;
    let visible_rows = (area.height as usize).saturating_sub(3); // borders + header

    // Build header row
    let mut header_cells = vec![TuiCell::from("").style(Style::default().fg(Color::Yellow))];
    for &(x, _) in &col_widths {
        header_cells.push(
            TuiCell::from(format!("{}", x))
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        );
    }
    let header = Row::new(header_cells).height(1);

    // Build data rows
    let mut rows = Vec::new();
    for dy in 0..visible_rows {
        let y = start_row + dy;
        let mut cells = vec![
            TuiCell::from(format!("{}", y))
                .style(Style::default().fg(Color::Yellow))
        ];

        for &(x, _) in &col_widths {
            let content = if let Some(cell) = sheet.get_cell(x, y, z) {
                cell.value.to_string()
            } else {
                String::new()
            };

            let style = if x == sheet.cur_x && y == sheet.cur_y {
                Style::default().fg(Color::Black).bg(Color::White)
            } else {
                Style::default()
            };

            cells.push(TuiCell::from(content).style(style));
        }

        rows.push(Row::new(cells).height(1));
    }

    // Build width constraints
    let mut constraints: Vec<Constraint> = vec![Constraint::Length(row_num_width)];
    for &(_, w) in &col_widths {
        constraints.push(Constraint::Length(w));
    }

    let table = Table::new(rows, constraints)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title("Teapot"))
        .column_spacing(1);

    f.render_widget(table, area);
}
