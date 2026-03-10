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
    text::{Line, Span},
    widgets::{Block, Borders, Cell as TuiCell, Clear, List, ListItem, Paragraph, Row, Table, Tabs},
    Frame, Terminal,
};

use regex::Regex;

use crate::scanner;
use crate::sheet::Sheet;
use crate::token::{Token, Operator};

/// All known commands for completion and palette
const COMMANDS: &[&str] = &[
    "q", "quit", "q!", "wq", "w", "write", "goto", "o", "open",
    "width", "precision", "bold", "underline", "align",
    "export-html", "export-latex", "export-context", "export-csv", "export-text",
    "clear", "copy", "move", "ir", "insert-row", "dr", "delete-row",
    "ic", "insert-col", "dc", "delete-col", "sort", "sort-x", "sort-y", "sort-z",
    "mirror-x", "mirror-y", "mirror-z", "fill",
    "undo", "redo", "yank", "paste", "help",
    "sheet", "sheet-add", "sheet-del", "sheets",
    "clock", "clock-run",
    "save-text",
    "search", "s", "search-all", "search-formula",
    "replace", "r", "replace-all",
];

struct DisplayState {
    status_message: String,
    input_mode: InputMode,
    input_buffer: String,
    cursor_position: usize,
    terminal_area: Rect,
    should_quit: bool,
    // Text editing: auto-wrap in quotes on commit
    text_editing: bool,
    // Command history
    command_history: Vec<String>,
    history_index: Option<usize>,
    history_stash: String,
    // Command completion
    completion_matches: Vec<String>,
    completion_index: usize,
    // Cell picker
    picker_x: usize,
    picker_y: usize,
    picker_z: usize,
    picker_active: bool,
    // Sheet picker
    sheet_picker_active: bool,
    sheet_picker_selection: usize,
    // Command palette
    palette_active: bool,
    palette_filter: String,
    palette_selection: usize,
    // Search/replace
    search_active: bool,
    search_pattern: String,
    search_results: Vec<(usize, usize, usize)>,
    search_index: usize,
    search_in_values: bool,
    search_regex: bool,
    replace_active: bool,
    replace_pattern: String,
    search_field: SearchField,
    replace_confirm: bool,
    search_all_layers: bool,
    // Go-to-ref state: origin cell and refs for cycling with g
    goto_ref_origin: Option<(usize, usize, usize)>,
    goto_ref_list: Vec<(usize, usize, usize)>,
    goto_ref_index: usize,
    // Dependents state: origin cell and deps for cycling with d
    goto_dep_origin: Option<(usize, usize, usize)>,
    goto_dep_list: Vec<(usize, usize, usize)>,
    goto_dep_index: usize,
}

#[derive(PartialEq, Clone, Copy)]
enum SearchField { Search, Replace }

#[derive(PartialEq)]
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
            should_quit: false,
            text_editing: false,
            command_history: Vec::new(),
            history_index: None,
            history_stash: String::new(),
            completion_matches: Vec::new(),
            completion_index: 0,
            picker_x: 0,
            picker_y: 0,
            picker_z: 0,
            picker_active: false,
            sheet_picker_active: false,
            sheet_picker_selection: 0,
            palette_active: false,
            palette_filter: String::new(),
            palette_selection: 0,
            search_active: false,
            search_pattern: String::new(),
            search_results: Vec::new(),
            search_index: 0,
            search_in_values: true,
            search_regex: true,
            replace_active: false,
            replace_pattern: String::new(),
            search_field: SearchField::Search,
            replace_confirm: false,
            search_all_layers: false,
            goto_ref_origin: None,
            goto_ref_list: Vec::new(),
            goto_ref_index: 0,
            goto_dep_origin: None,
            goto_dep_list: Vec::new(),
            goto_dep_index: 0,
        }
    }
}

/// Insert a character at the cursor position in the input buffer
fn buffer_insert(buf: &mut String, pos: &mut usize, c: char) {
    if *pos >= buf.len() {
        buf.push(c);
    } else {
        buf.insert(*pos, c);
    }
    *pos += 1;
}

/// Delete a character before the cursor (backspace)
fn buffer_backspace(buf: &mut String, pos: &mut usize) {
    if *pos > 0 && !buf.is_empty() {
        *pos -= 1;
        if *pos < buf.len() {
            buf.remove(*pos);
        }
    }
}

/// Delete a character at the cursor (delete key)
fn buffer_delete(buf: &mut String, pos: usize) {
    if pos < buf.len() {
        buf.remove(pos);
    }
}

/// Delete word backwards (Ctrl+W)
fn buffer_kill_word_back(buf: &mut String, pos: &mut usize) {
    if *pos == 0 { return; }
    // Skip trailing spaces
    let mut new_pos = *pos;
    while new_pos > 0 && buf.as_bytes().get(new_pos - 1) == Some(&b' ') {
        new_pos -= 1;
    }
    // Skip word chars
    while new_pos > 0 && buf.as_bytes().get(new_pos - 1) != Some(&b' ') {
        new_pos -= 1;
    }
    buf.drain(new_pos..*pos);
    *pos = new_pos;
}

/// Kill to end of line (Ctrl+K)
fn buffer_kill_to_end(buf: &mut String, pos: usize) {
    buf.truncate(pos);
}

/// Handle readline-style shortcuts common to Editing and Command modes.
/// Returns true if the key was handled.
fn handle_readline(key: &crossterm::event::KeyEvent, buf: &mut String, pos: &mut usize, min_pos: usize) -> bool {
    match key.code {
        KeyCode::Left | KeyCode::Char('b') if key.code == KeyCode::Left || key.modifiers.contains(KeyModifiers::CONTROL) => {
            if *pos > min_pos { *pos -= 1; }
            true
        }
        KeyCode::Right | KeyCode::Char('f') if key.code == KeyCode::Right || key.modifiers.contains(KeyModifiers::CONTROL) => {
            if *pos < buf.len() { *pos += 1; }
            true
        }
        KeyCode::Home => {
            *pos = min_pos;
            true
        }
        KeyCode::End => {
            *pos = buf.len();
            true
        }
        KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            *pos = min_pos;
            true
        }
        KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            *pos = buf.len();
            true
        }
        KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            buffer_kill_to_end(buf, *pos);
            true
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let kill_start = min_pos;
            buf.drain(kill_start..*pos);
            *pos = kill_start;
            true
        }
        KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Don't kill back past min_pos
            let old_pos = *pos;
            buffer_kill_word_back(buf, pos);
            if *pos < min_pos { *pos = min_pos; }
            let _ = old_pos; // suppress warning
            true
        }
        KeyCode::Delete => {
            buffer_delete(buf, *pos);
            true
        }
        KeyCode::Backspace => {
            if *pos > min_pos {
                buffer_backspace(buf, pos);
            }
            true
        }
        _ => false,
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

/// Commit editing buffer to the sheet cell
fn commit_edit(sheet: &mut Sheet, state: &mut DisplayState) {
    sheet.save_undo();
    let mut input = state.input_buffer.trim().to_string();
    let (x, y, z) = (sheet.cur_x, sheet.cur_y, sheet.cur_z);

    // Text editing mode: wrap in quotes
    if state.text_editing && !input.is_empty() {
        input = format!("\"{}\"", input);
    }

    if input.is_empty() {
        if let Some(cell) = sheet.get_cell_mut(x, y, z) {
            cell.contents = None;
            cell.value = Token::Empty;
        }
        state.status_message = format!("Cell ({},{},{}) cleared", x, y, z);
    } else {
        match scanner::scan(&input) {
            Ok(tokens) => {
                sheet.putcont(x, y, z, tokens);
                // Set left alignment for text editing
                if state.text_editing {
                    if let Some(cell) = sheet.get_cell_mut(x, y, z) {
                        cell.adjust = crate::sheet::Adjust::Left;
                    }
                }
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
    state.text_editing = false;
    state.input_mode = InputMode::Normal;
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
            // Handle search overlay first
            if state.search_active {
                handle_search(key, sheet, state);
                continue;
            }
            // Handle replace confirmation
            if state.replace_confirm {
                handle_replace_confirm(key, sheet, state);
                continue;
            }
            // Handle palette mode first (overlays everything)
            if state.palette_active {
                handle_palette(key, sheet, state);
                continue;
            }
            // Handle sheet picker
            if state.sheet_picker_active {
                handle_sheet_picker(key, sheet, state);
                continue;
            }
            // Handle cell picker
            if state.picker_active {
                handle_cell_picker(key, sheet, state);
                continue;
            }

            match state.input_mode {
                InputMode::Normal => handle_normal(key, sheet, state),
                InputMode::Editing => handle_editing(key, sheet, state),
                InputMode::Command => handle_command(key, sheet, state),
                InputMode::Help => {
                    state.input_mode = InputMode::Normal;
                }
            }
        }
        if state.should_quit {
            return Ok(());
        }
    }
}

fn handle_normal(key: crossterm::event::KeyEvent, sheet: &mut Sheet, state: &mut DisplayState) {
    // Clear goto-ref cycle on any key other than g
    if key.code != KeyCode::Char('g') && state.goto_ref_origin.is_some() {
        state.goto_ref_origin = None;
        state.goto_ref_list.clear();
        state.goto_ref_index = 0;
    }
    // Clear goto-dep cycle on any key other than d
    if key.code != KeyCode::Char('d') && state.goto_dep_origin.is_some() {
        state.goto_dep_origin = None;
        state.goto_dep_list.clear();
        state.goto_dep_index = 0;
    }
    match key.code {
        KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.should_quit = true;
            return;
        }
        KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if sheet.undo() {
                state.status_message = String::from("Undone");
            } else {
                state.status_message = String::from("Nothing to undo");
            }
        }
        KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if sheet.redo() {
                state.status_message = String::from("Redone");
            } else {
                state.status_message = String::from("Nothing to redo");
            }
        }
        // Smart entry: = for formula
        KeyCode::Char('=') => {
            state.input_mode = InputMode::Editing;
            state.text_editing = false;
            state.input_buffer.clear();
            state.cursor_position = 0;
        }
        // Smart entry: ' for text
        KeyCode::Char('\'') => {
            state.input_mode = InputMode::Editing;
            state.text_editing = true;
            state.input_buffer.clear();
            state.cursor_position = 0;
        }
        // Smart entry: digits for quick number entry
        KeyCode::Char(c @ '0'..='9') => {
            state.input_mode = InputMode::Editing;
            state.text_editing = false;
            state.input_buffer.clear();
            state.input_buffer.push(c);
            state.cursor_position = 1;
        }
        KeyCode::Char('e') | KeyCode::Enter => {
            state.input_mode = InputMode::Editing;
            state.text_editing = false;
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
            state.history_index = None;
            state.completion_matches.clear();
        }
        // Command palette
        KeyCode::Char('/') | KeyCode::F(1) => {
            state.palette_active = true;
            state.palette_filter.clear();
            state.palette_selection = 0;
        }
        // Navigation
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
        // Sheet navigation
        KeyCode::Tab => {
            if sheet.cur_z + 1 < sheet.dim_z {
                sheet.cur_z += 1;
            } else {
                sheet.cur_z = 0;
            }
        }
        KeyCode::Char('[') => {
            if sheet.cur_z > 0 { sheet.cur_z -= 1; }
        }
        KeyCode::Char(']') => {
            if sheet.cur_z + 1 < sheet.dim_z { sheet.cur_z += 1; }
        }
        KeyCode::Char('Z') => {
            // Sheet picker
            state.sheet_picker_active = true;
            state.sheet_picker_selection = sheet.cur_z;
        }
        // Search
        KeyCode::Char('n') if !state.search_results.is_empty() => {
            if state.search_results.len() > 1 {
                state.search_index = (state.search_index + 1) % state.search_results.len();
            }
            let (x, y, z) = state.search_results[state.search_index];
            sheet.cur_x = x;
            sheet.cur_y = y;
            sheet.cur_z = z;
            state.status_message = format!("Match {}/{}", state.search_index + 1, state.search_results.len());
        }
        KeyCode::Char('n') => {
            // Open search bar
            state.search_active = true;
            state.search_field = SearchField::Search;
            state.replace_active = false;
            state.search_pattern.clear();
            state.search_results.clear();
            state.search_index = 0;
        }
        KeyCode::Char('N') => {
            if !state.search_results.is_empty() {
                if state.search_index == 0 {
                    state.search_index = state.search_results.len() - 1;
                } else {
                    state.search_index -= 1;
                }
                let (x, y, z) = state.search_results[state.search_index];
                sheet.cur_x = x;
                sheet.cur_y = y;
                sheet.cur_z = z;
                state.status_message = format!("Match {}/{}", state.search_index + 1, state.search_results.len());
            }
        }
        KeyCode::Char('r') => {
            // Open search+replace
            state.search_active = true;
            state.search_field = SearchField::Search;
            state.replace_active = true;
            state.search_pattern.clear();
            state.replace_pattern.clear();
            state.search_results.clear();
            state.search_index = 0;
        }
        KeyCode::Char('g') => {
            let cur = (sheet.cur_x, sheet.cur_y, sheet.cur_z);
            // If we're already cycling refs from a previous g press, advance
            if state.goto_ref_origin.is_some() && !state.goto_ref_list.is_empty() {
                // Check if we're still on one of the refs or the origin
                let on_ref = state.goto_ref_list.contains(&cur);
                let on_origin = state.goto_ref_origin == Some(cur);
                if on_ref || on_origin {
                    state.goto_ref_index = (state.goto_ref_index + 1) % (state.goto_ref_list.len() + 1);
                    if state.goto_ref_index == state.goto_ref_list.len() {
                        // Cycle back to origin
                        let (ox, oy, oz) = state.goto_ref_origin.unwrap();
                        sheet.cur_x = ox;
                        sheet.cur_y = oy;
                        sheet.cur_z = oz;
                        state.status_message = format!("Back to origin ({},{},{})", ox, oy, oz);
                    } else {
                        let (x, y, z) = state.goto_ref_list[state.goto_ref_index];
                        sheet.cur_x = x;
                        sheet.cur_y = y;
                        sheet.cur_z = z;
                        state.status_message = format!("Ref {}/{}: @({},{},{})",
                            state.goto_ref_index + 1, state.goto_ref_list.len(), x, y, z);
                    }
                } else {
                    // Moved away, start fresh from current cell
                    state.goto_ref_origin = None;
                    state.goto_ref_list.clear();
                }
            }
            // Start fresh if no active cycle
            if state.goto_ref_origin.is_none() {
                let refs = sheet
                    .get_cell(sheet.cur_x, sheet.cur_y, sheet.cur_z)
                    .and_then(|c| c.contents.as_ref())
                    .map(|tokens| extract_cell_refs(tokens))
                    .unwrap_or_default();
                if refs.is_empty() {
                    state.status_message = String::from("No cell references in current cell");
                } else {
                    state.goto_ref_origin = Some(cur);
                    state.goto_ref_list = refs;
                    state.goto_ref_index = 0;
                    let (x, y, z) = state.goto_ref_list[0];
                    sheet.cur_x = x;
                    sheet.cur_y = y;
                    sheet.cur_z = z;
                    state.status_message = format!("Ref 1/{}: @({},{},{})",
                        state.goto_ref_list.len(), x, y, z);
                }
            }
        }
        KeyCode::Char('d') => {
            let cur = (sheet.cur_x, sheet.cur_y, sheet.cur_z);
            // If already cycling deps, advance
            if state.goto_dep_origin.is_some() && !state.goto_dep_list.is_empty() {
                let on_dep = state.goto_dep_list.contains(&cur);
                let on_origin = state.goto_dep_origin == Some(cur);
                if on_dep || on_origin {
                    state.goto_dep_index = (state.goto_dep_index + 1) % (state.goto_dep_list.len() + 1);
                    if state.goto_dep_index == state.goto_dep_list.len() {
                        let (ox, oy, oz) = state.goto_dep_origin.unwrap();
                        sheet.cur_x = ox;
                        sheet.cur_y = oy;
                        sheet.cur_z = oz;
                        state.status_message = format!("Back to origin ({},{},{})", ox, oy, oz);
                    } else {
                        let (x, y, z) = state.goto_dep_list[state.goto_dep_index];
                        sheet.cur_x = x;
                        sheet.cur_y = y;
                        sheet.cur_z = z;
                        state.status_message = format!("Dep {}/{}: ({},{},{})",
                            state.goto_dep_index + 1, state.goto_dep_list.len(), x, y, z);
                    }
                } else {
                    state.goto_dep_origin = None;
                    state.goto_dep_list.clear();
                }
            }
            // Start fresh if no active cycle
            if state.goto_dep_origin.is_none() {
                let deps = find_dependents(sheet, sheet.cur_x, sheet.cur_y, sheet.cur_z);
                if deps.is_empty() {
                    state.status_message = String::from("No cells depend on this cell");
                } else {
                    state.goto_dep_origin = Some(cur);
                    state.goto_dep_list = deps;
                    state.goto_dep_index = 0;
                    let (x, y, z) = state.goto_dep_list[0];
                    sheet.cur_x = x;
                    sheet.cur_y = y;
                    sheet.cur_z = z;
                    state.status_message = format!("Dep 1/{}: ({},{},{})",
                        state.goto_dep_list.len(), x, y, z);
                }
            }
        }
        KeyCode::Esc => {
            // Clear search results
            state.search_results.clear();
            state.search_index = 0;
        }
        KeyCode::Char('?') => {
            state.input_mode = InputMode::Help;
        }
        KeyCode::Char('+') => {
            let w = sheet.column_width(sheet.cur_x, sheet.cur_z);
            sheet.set_width(sheet.cur_x, sheet.cur_z, w + 1);
            sheet.changed = true;
            state.status_message = format!("Column {} width: {}", sheet.cur_x, w + 1);
        }
        KeyCode::Char('-') => {
            let w = sheet.column_width(sheet.cur_x, sheet.cur_z);
            if w > 1 {
                sheet.set_width(sheet.cur_x, sheet.cur_z, w - 1);
                sheet.changed = true;
                state.status_message = format!("Column {} width: {}", sheet.cur_x, w - 1);
            }
        }
        KeyCode::Char('m') => {
            if !sheet.marking {
                sheet.mark1_x = Some(sheet.cur_x);
                sheet.mark1_y = Some(sheet.cur_y);
                sheet.mark1_z = Some(sheet.cur_z);
                sheet.marking = true;
                state.status_message = format!("Mark1 set at ({},{},{}). Move and press m again.",
                    sheet.cur_x, sheet.cur_y, sheet.cur_z);
            } else {
                sheet.mark2_x = Some(sheet.cur_x);
                sheet.mark2_y = Some(sheet.cur_y);
                sheet.mark2_z = Some(sheet.cur_z);
                sheet.marking = false;
                state.status_message = format!("Block marked ({},{},{}) to ({},{},{})",
                    sheet.mark1_x.unwrap_or(0), sheet.mark1_y.unwrap_or(0), sheet.mark1_z.unwrap_or(0),
                    sheet.cur_x, sheet.cur_y, sheet.cur_z);
            }
        }
        KeyCode::Char('u') => {
            sheet.clear_mark();
            state.status_message = String::from("Mark cleared");
        }
        KeyCode::Char('y') => {
            let count = sheet.yank_block();
            if count > 0 {
                state.status_message = format!("Yanked {} cells", count);
            } else {
                state.status_message = String::from("No block marked to yank");
            }
        }
        KeyCode::Char('p') => {
            if sheet.clipboard.is_empty() {
                state.status_message = String::from("Clipboard empty");
            } else {
                let count = sheet.paste();
                state.status_message = format!("Pasted {} cells", count);
            }
        }
        // Clock tick
        KeyCode::Char('C') => {
            let count = sheet.clock_tick();
            if count > 0 {
                state.status_message = format!("Clock tick: {} cells updated", count);
            } else {
                state.status_message = String::from("No clocked cells");
            }
        }
        KeyCode::Home => {
            sheet.cur_x = 0;
            sheet.cur_y = 0;
        }
        KeyCode::End => {
            sheet.cur_x = sheet.dim_x.saturating_sub(1);
            sheet.cur_y = sheet.dim_y.saturating_sub(1);
        }
        KeyCode::PageUp => {
            let page = visible_rows(state.terminal_area);
            sheet.cur_y = sheet.cur_y.saturating_sub(page);
        }
        KeyCode::PageDown => {
            let page = visible_rows(state.terminal_area);
            sheet.cur_y += page;
        }
        KeyCode::Delete => {
            sheet.save_undo();
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

fn handle_editing(key: crossterm::event::KeyEvent, sheet: &mut Sheet, state: &mut DisplayState) {
    match key.code {
        KeyCode::Enter => {
            commit_edit(sheet, state);
        }
        KeyCode::Esc => {
            state.text_editing = false;
            state.input_mode = InputMode::Normal;
        }
        // @ triggers cell picker
        KeyCode::Char('@') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Enter cell picker mode
            state.picker_active = true;
            state.picker_x = sheet.cur_x;
            state.picker_y = sheet.cur_y;
            state.picker_z = sheet.cur_z;
        }
        _ => {
            // Try readline shortcuts first (min_pos = 0 for editing)
            if !handle_readline(&key, &mut state.input_buffer, &mut state.cursor_position, 0) {
                // Regular character input
                if let KeyCode::Char(c) = key.code {
                    if !key.modifiers.contains(KeyModifiers::CONTROL) {
                        buffer_insert(&mut state.input_buffer, &mut state.cursor_position, c);
                    }
                }
            }
        }
    }
}

fn handle_command(key: crossterm::event::KeyEvent, sheet: &mut Sheet, state: &mut DisplayState) {
    match key.code {
        KeyCode::Enter => {
            let cmd_str = state.input_buffer.clone();
            // Push to history if non-empty and not just ":"
            if cmd_str.len() > 1 {
                // Don't duplicate last entry
                if state.command_history.last().map(|s| s.as_str()) != Some(&cmd_str) {
                    state.command_history.push(cmd_str);
                    if state.command_history.len() > 100 {
                        state.command_history.remove(0);
                    }
                }
            }
            state.history_index = None;
            state.completion_matches.clear();
            process_command(sheet, state);
            if state.input_mode != InputMode::Help {
                state.input_mode = InputMode::Normal;
            }
        }
        KeyCode::Esc => {
            state.history_index = None;
            state.completion_matches.clear();
            state.input_mode = InputMode::Normal;
        }
        // History navigation
        KeyCode::Up => {
            if state.command_history.is_empty() { return; }
            match state.history_index {
                None => {
                    state.history_stash = state.input_buffer.clone();
                    let idx = state.command_history.len() - 1;
                    state.history_index = Some(idx);
                    state.input_buffer = state.command_history[idx].clone();
                    state.cursor_position = state.input_buffer.len();
                }
                Some(idx) if idx > 0 => {
                    let idx = idx - 1;
                    state.history_index = Some(idx);
                    state.input_buffer = state.command_history[idx].clone();
                    state.cursor_position = state.input_buffer.len();
                }
                _ => {}
            }
        }
        KeyCode::Down => {
            if let Some(idx) = state.history_index {
                if idx + 1 < state.command_history.len() {
                    let idx = idx + 1;
                    state.history_index = Some(idx);
                    state.input_buffer = state.command_history[idx].clone();
                    state.cursor_position = state.input_buffer.len();
                } else {
                    state.history_index = None;
                    state.input_buffer = state.history_stash.clone();
                    state.cursor_position = state.input_buffer.len();
                }
            }
        }
        // Tab completion
        KeyCode::Tab => {
            let prefix = state.input_buffer.trim_start_matches(':');
            if state.completion_matches.is_empty() {
                // Build matches
                state.completion_matches = COMMANDS.iter()
                    .filter(|cmd| cmd.starts_with(prefix))
                    .map(|s| s.to_string())
                    .collect();
                state.completion_index = 0;
            } else {
                state.completion_index = (state.completion_index + 1) % state.completion_matches.len();
            }
            if !state.completion_matches.is_empty() {
                let completed = &state.completion_matches[state.completion_index];
                state.input_buffer = format!(":{}", completed);
                state.cursor_position = state.input_buffer.len();
                // Show available completions in status
                let completions: Vec<&str> = state.completion_matches.iter().map(|s| s.as_str()).collect();
                state.status_message = format!("Completions: {}", completions.join(", "));
            }
        }
        KeyCode::BackTab => {
            if !state.completion_matches.is_empty() {
                if state.completion_index == 0 {
                    state.completion_index = state.completion_matches.len() - 1;
                } else {
                    state.completion_index -= 1;
                }
                let completed = &state.completion_matches[state.completion_index];
                state.input_buffer = format!(":{}", completed);
                state.cursor_position = state.input_buffer.len();
            }
        }
        _ => {
            // Clear completion on any other key
            state.completion_matches.clear();
            // Try readline shortcuts (min_pos = 1 to keep the ':')
            if !handle_readline(&key, &mut state.input_buffer, &mut state.cursor_position, 1) {
                if let KeyCode::Char(c) = key.code {
                    if !key.modifiers.contains(KeyModifiers::CONTROL) {
                        buffer_insert(&mut state.input_buffer, &mut state.cursor_position, c);
                    }
                }
            }
        }
    }
}

fn handle_cell_picker(key: crossterm::event::KeyEvent, sheet: &mut Sheet, state: &mut DisplayState) {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if state.picker_y > 0 { state.picker_y -= 1; }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.picker_y += 1;
        }
        KeyCode::Left | KeyCode::Char('h') => {
            if state.picker_x > 0 { state.picker_x -= 1; }
        }
        KeyCode::Right | KeyCode::Char('l') => {
            state.picker_x += 1;
        }
        KeyCode::Tab => {
            // Cycle z in picker
            if state.picker_z + 1 < sheet.dim_z.max(1) {
                state.picker_z += 1;
            } else {
                state.picker_z = 0;
            }
        }
        KeyCode::Enter => {
            // Insert @(x,y,z) at cursor
            let reference = format!("@({},{},{})", state.picker_x, state.picker_y, state.picker_z);
            for c in reference.chars() {
                buffer_insert(&mut state.input_buffer, &mut state.cursor_position, c);
            }
            state.picker_active = false;
        }
        KeyCode::Esc => {
            state.picker_active = false;
        }
        _ => {}
    }
    // Keep viewport adjusted for picker position
    let _ = sheet;
}

fn handle_sheet_picker(key: crossterm::event::KeyEvent, sheet: &mut Sheet, state: &mut DisplayState) {
    let max_z = sheet.dim_z.max(1);
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if state.sheet_picker_selection > 0 {
                state.sheet_picker_selection -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if state.sheet_picker_selection + 1 < max_z {
                state.sheet_picker_selection += 1;
            }
        }
        KeyCode::Enter => {
            sheet.cur_z = state.sheet_picker_selection;
            state.sheet_picker_active = false;
            state.status_message = format!("Switched to Sheet {}", sheet.cur_z + 1);
        }
        KeyCode::Esc => {
            state.sheet_picker_active = false;
        }
        _ => {}
    }
}

/// Run incremental search and update results
fn update_search_results(sheet: &Sheet, state: &mut DisplayState) {
    if state.search_pattern.is_empty() {
        state.search_results.clear();
        state.search_index = 0;
        return;
    }
    let re = if state.search_regex {
        match Regex::new(&state.search_pattern) {
            Ok(r) => r,
            Err(_) => {
                // Fall back to literal match
                match Regex::new(&regex::escape(&state.search_pattern)) {
                    Ok(r) => r,
                    Err(_) => return,
                }
            }
        }
    } else {
        match Regex::new(&regex::escape(&state.search_pattern)) {
            Ok(r) => r,
            Err(_) => return,
        }
    };

    state.search_results = if state.search_all_layers {
        sheet.search_all_cells(&re, state.search_in_values)
    } else {
        sheet.search_cells(sheet.cur_z, &re, state.search_in_values)
    };
    if state.search_index >= state.search_results.len() {
        state.search_index = 0;
    }
}

fn handle_search(key: crossterm::event::KeyEvent, sheet: &mut Sheet, state: &mut DisplayState) {
    match state.search_field {
        SearchField::Search => {
            match key.code {
                KeyCode::Esc => {
                    state.search_active = false;
                    state.search_results.clear();
                    state.search_index = 0;
                }
                KeyCode::Enter => {
                    if !state.search_results.is_empty() {
                        let (x, y, z) = state.search_results[state.search_index];
                        sheet.cur_x = x;
                        sheet.cur_y = y;
                        sheet.cur_z = z;
                        state.status_message = format!("Match {}/{}", state.search_index + 1, state.search_results.len());
                        adjust_viewport(sheet, state.terminal_area);
                    } else {
                        state.status_message = String::from("No matches");
                    }
                    state.search_active = false;
                }
                KeyCode::Tab => {
                    if state.replace_active {
                        state.search_field = SearchField::Replace;
                    }
                }
                KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    // Toggle search in values vs formulas
                    state.search_in_values = !state.search_in_values;
                    update_search_results(sheet, state);
                    let what = if state.search_in_values { "values" } else { "formulas" };
                    state.status_message = format!("Searching {}", what);
                }
                KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    // Toggle regex mode
                    state.search_regex = !state.search_regex;
                    update_search_results(sheet, state);
                    let mode = if state.search_regex { "regex" } else { "literal" };
                    state.status_message = format!("Search mode: {}", mode);
                }
                KeyCode::Backspace => {
                    state.search_pattern.pop();
                    update_search_results(sheet, state);
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    state.search_pattern.push(c);
                    update_search_results(sheet, state);
                }
                _ => {}
            }
        }
        SearchField::Replace => {
            match key.code {
                KeyCode::Esc => {
                    state.search_active = false;
                    state.search_results.clear();
                    state.search_index = 0;
                }
                KeyCode::Tab | KeyCode::BackTab => {
                    state.search_field = SearchField::Search;
                }
                KeyCode::Enter => {
                    // Start replace confirmation
                    if !state.search_results.is_empty() {
                        let (x, y, z) = state.search_results[state.search_index];
                        sheet.cur_x = x;
                        sheet.cur_y = y;
                        sheet.cur_z = z;
                        adjust_viewport(sheet, state.terminal_area);
                        state.search_active = false;
                        state.replace_confirm = true;
                        state.status_message = format!(
                            "Replace? (y)es (n)o (a)ll (q)uit — Match {}/{}",
                            state.search_index + 1, state.search_results.len()
                        );
                    } else {
                        state.status_message = String::from("No matches to replace");
                        state.search_active = false;
                    }
                }
                KeyCode::Backspace => {
                    state.replace_pattern.pop();
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    state.replace_pattern.push(c);
                }
                _ => {}
            }
        }
    }
}

fn handle_replace_confirm(key: crossterm::event::KeyEvent, sheet: &mut Sheet, state: &mut DisplayState) {
    let re = if state.search_regex {
        Regex::new(&state.search_pattern).unwrap_or_else(|_| Regex::new(&regex::escape(&state.search_pattern)).unwrap())
    } else {
        Regex::new(&regex::escape(&state.search_pattern)).unwrap()
    };

    match key.code {
        KeyCode::Char('y') => {
            sheet.save_undo();
            let (x, y, z) = state.search_results[state.search_index];
            sheet.replace_cell(x, y, z, &re, &state.replace_pattern, state.search_in_values);
            sheet.update();
            // Re-run search to refresh results
            update_search_results(sheet, state);
            if state.search_results.is_empty() {
                state.replace_confirm = false;
                state.status_message = String::from("All matches replaced");
            } else {
                if state.search_index >= state.search_results.len() {
                    state.search_index = 0;
                }
                let (x, y, z) = state.search_results[state.search_index];
                sheet.cur_x = x;
                sheet.cur_y = y;
                sheet.cur_z = z;
                adjust_viewport(sheet, state.terminal_area);
                state.status_message = format!(
                    "Replaced. Next? (y/n/a/q) — Match {}/{}",
                    state.search_index + 1, state.search_results.len()
                );
            }
        }
        KeyCode::Char('n') => {
            // Skip this match
            if state.search_results.len() > 1 {
                state.search_index = (state.search_index + 1) % state.search_results.len();
                let (x, y, z) = state.search_results[state.search_index];
                sheet.cur_x = x;
                sheet.cur_y = y;
                sheet.cur_z = z;
                adjust_viewport(sheet, state.terminal_area);
                state.status_message = format!(
                    "Replace? (y/n/a/q) — Match {}/{}",
                    state.search_index + 1, state.search_results.len()
                );
            } else {
                state.replace_confirm = false;
                state.status_message = String::from("No more matches");
            }
        }
        KeyCode::Char('a') => {
            // Replace all
            sheet.save_undo();
            let mut count = 0;
            for &(x, y, z) in &state.search_results {
                if sheet.replace_cell(x, y, z, &re, &state.replace_pattern, state.search_in_values) {
                    count += 1;
                }
            }
            sheet.update();
            state.search_results.clear();
            state.search_index = 0;
            state.replace_confirm = false;
            state.status_message = format!("Replaced {} matches", count);
        }
        KeyCode::Char('q') | KeyCode::Esc => {
            state.replace_confirm = false;
            state.status_message = String::from("Replace cancelled");
        }
        _ => {}
    }
}

fn handle_palette(key: crossterm::event::KeyEvent, sheet: &mut Sheet, state: &mut DisplayState) {
    let filtered = get_palette_items(&state.palette_filter);
    match key.code {
        KeyCode::Esc => {
            state.palette_active = false;
        }
        KeyCode::Enter => {
            if let Some(cmd) = filtered.get(state.palette_selection) {
                state.input_buffer = format!(":{}", cmd);
                state.cursor_position = state.input_buffer.len();
                state.palette_active = false;
                process_command(sheet, state);
                if state.input_mode != InputMode::Help {
                    state.input_mode = InputMode::Normal;
                }
            } else {
                state.palette_active = false;
            }
        }
        KeyCode::Up => {
            if state.palette_selection > 0 {
                state.palette_selection -= 1;
            }
        }
        KeyCode::Down => {
            if state.palette_selection + 1 < filtered.len() {
                state.palette_selection += 1;
            }
        }
        KeyCode::Backspace => {
            state.palette_filter.pop();
            state.palette_selection = 0;
        }
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.palette_filter.push(c);
            state.palette_selection = 0;
        }
        _ => {}
    }
}

/// Get filtered palette items (fuzzy substring match)
fn get_palette_items(filter: &str) -> Vec<&'static str> {
    if filter.is_empty() {
        return COMMANDS.to_vec();
    }
    let filter_lower = filter.to_lowercase();
    COMMANDS.iter()
        .filter(|cmd| cmd.to_lowercase().contains(&filter_lower))
        .copied()
        .collect()
}

/// Save a sheet using the appropriate format based on file extension
fn save_by_extension(sheet: &Sheet, filename: &str) -> anyhow::Result<usize> {
    if filename.ends_with(".tpz") {
        crate::fileio::save_tpz(sheet, filename)
    } else if filename.ends_with(".xlsx") {
        crate::fileio::xlsx::save_xlsx(sheet, filename)
    } else {
        crate::fileio::save_port(sheet, filename)
    }
}

fn process_command(sheet: &mut Sheet, state: &mut DisplayState) {
    let cmd_owned = state.input_buffer.trim_start_matches(':').trim().to_string();
    let parts: Vec<&str> = cmd_owned.splitn(2, ' ').collect();
    let command = parts[0];
    let arg = parts.get(1).map(|s| s.trim()).unwrap_or("");

    match command {
        "q" | "quit" => {
            if sheet.changed {
                state.status_message = String::from("Unsaved changes. Use :q! to force quit, or :wq to save and quit.");
            } else {
                state.should_quit = true;
            }
        }
        "q!" => {
            state.should_quit = true;
        }
        "wq" => {
            let filename = sheet.name.clone().unwrap_or_else(|| "sheet.tp".to_string());
            match save_by_extension(sheet, &filename) {
                Ok(count) => {
                    sheet.name = Some(filename.clone());
                    sheet.changed = false;
                    state.status_message = format!("Saved {} cells to {}", count, filename);
                    state.should_quit = true;
                }
                Err(e) => {
                    state.status_message = format!("Save failed: {}", e);
                }
            }
        }
        "w" | "write" => {
            let filename = if arg.is_empty() {
                sheet.name.clone().unwrap_or_else(|| "sheet.tp".to_string())
            } else {
                arg.to_string()
            };
            match save_by_extension(sheet, &filename) {
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
        "o" | "open" => {
            if arg.is_empty() {
                state.status_message = String::from("Usage: :open <file>");
            } else {
                let path = std::path::Path::new(arg);
                let mut new_sheet = Sheet::new();
                match crate::fileio::load_file(&mut new_sheet, path, true) {
                    Ok(()) => {
                        new_sheet.name = Some(arg.to_string());
                        *sheet = new_sheet;
                        state.status_message = format!("Opened {}", arg);
                    }
                    Err(e) => {
                        state.status_message = format!("Open failed: {}", e);
                    }
                }
            }
        }
        "width" => {
            if arg.is_empty() {
                let w = sheet.column_width(sheet.cur_x, sheet.cur_z);
                state.status_message = format!("Column {} width: {}", sheet.cur_x, w);
            } else if let Ok(w) = arg.parse::<usize>() {
                if w > 0 {
                    sheet.set_width(sheet.cur_x, sheet.cur_z, w);
                    sheet.changed = true;
                    state.status_message = format!("Column {} width set to {}", sheet.cur_x, w);
                } else {
                    state.status_message = String::from("Width must be > 0");
                }
            } else {
                state.status_message = String::from("Usage: :width <number>");
            }
        }
        "precision" => {
            if let Ok(p) = arg.parse::<i32>() {
                let (x, y, z) = (sheet.cur_x, sheet.cur_y, sheet.cur_z);
                let cell = sheet.get_or_create_cell(x, y, z);
                cell.precision = p;
                sheet.changed = true;
                state.status_message = format!("Precision set to {}", p);
            } else {
                state.status_message = String::from("Usage: :precision <number>");
            }
        }
        "bold" => {
            let (x, y, z) = (sheet.cur_x, sheet.cur_y, sheet.cur_z);
            let cell = sheet.get_or_create_cell(x, y, z);
            cell.bold = !cell.bold;
            let val = cell.bold;
            sheet.changed = true;
            state.status_message = format!("Bold: {}", val);
        }
        "underline" => {
            let (x, y, z) = (sheet.cur_x, sheet.cur_y, sheet.cur_z);
            let cell = sheet.get_or_create_cell(x, y, z);
            cell.underline = !cell.underline;
            let val = cell.underline;
            sheet.changed = true;
            state.status_message = format!("Underline: {}", val);
        }
        "align" => {
            let (x, y, z) = (sheet.cur_x, sheet.cur_y, sheet.cur_z);
            let cell = sheet.get_or_create_cell(x, y, z);
            match arg {
                "left" | "l" => { cell.adjust = crate::sheet::Adjust::Left; }
                "right" | "r" => { cell.adjust = crate::sheet::Adjust::Right; }
                "center" | "c" => { cell.adjust = crate::sheet::Adjust::Center; }
                "auto" | "a" => { cell.adjust = crate::sheet::Adjust::AutoAdjust; }
                _ => {
                    state.status_message = String::from("Usage: :align left|right|center|auto");
                    return;
                }
            }
            let val = format!("{:?}", cell.adjust);
            sheet.changed = true;
            state.status_message = format!("Alignment: {}", val);
        }
        "export-html" => {
            if arg.is_empty() {
                state.status_message = String::from("Usage: :export-html <file>");
            } else {
                match crate::fileio::save_html(sheet, arg, false,
                    0, 0, 0, sheet.dim_x.saturating_sub(1), sheet.dim_y.saturating_sub(1), sheet.dim_z.saturating_sub(1)) {
                    Ok(count) => state.status_message = format!("Exported {} cells to {}", count, arg),
                    Err(e) => state.status_message = format!("Export failed: {}", e),
                }
            }
        }
        "export-latex" => {
            if arg.is_empty() {
                state.status_message = String::from("Usage: :export-latex <file>");
            } else {
                match crate::fileio::save_latex(sheet, arg, false,
                    0, 0, 0, sheet.dim_x.saturating_sub(1), sheet.dim_y.saturating_sub(1), sheet.dim_z.saturating_sub(1)) {
                    Ok(count) => state.status_message = format!("Exported {} cells to {}", count, arg),
                    Err(e) => state.status_message = format!("Export failed: {}", e),
                }
            }
        }
        "export-context" => {
            if arg.is_empty() {
                state.status_message = String::from("Usage: :export-context <file>");
            } else {
                match crate::fileio::save_context(sheet, arg, false,
                    0, 0, 0, sheet.dim_x.saturating_sub(1), sheet.dim_y.saturating_sub(1), sheet.dim_z.saturating_sub(1)) {
                    Ok(count) => state.status_message = format!("Exported {} cells to {}", count, arg),
                    Err(e) => state.status_message = format!("Export failed: {}", e),
                }
            }
        }
        "export-csv" => {
            if arg.is_empty() {
                state.status_message = String::from("Usage: :export-csv <file>");
            } else {
                match crate::fileio::save_csv(sheet, arg, ',',
                    0, 0, 0, sheet.dim_x.saturating_sub(1), sheet.dim_y.saturating_sub(1), 0) {
                    Ok(count) => state.status_message = format!("Exported {} cells to {}", count, arg),
                    Err(e) => state.status_message = format!("Export failed: {}", e),
                }
            }
        }
        "clear" => {
            if let Some((x1, y1, z1, x2, y2, z2)) = sheet.get_mark_range() {
                sheet.save_undo();
                let count = sheet.clear_block(x1, y1, z1, x2, y2, z2);
                sheet.clear_mark();
                sheet.update();
                state.status_message = format!("Cleared {} cells", count);
            } else {
                state.status_message = String::from("No block marked. Press m twice to mark a block.");
            }
        }
        "copy" => {
            if let Some((x1, y1, z1, x2, y2, z2)) = sheet.get_mark_range() {
                sheet.save_undo();
                let count = sheet.copy_block(x1, y1, z1, x2, y2, z2,
                    sheet.cur_x, sheet.cur_y, sheet.cur_z);
                sheet.clear_mark();
                sheet.update();
                state.status_message = format!("Copied {} cells to ({},{},{})",
                    count, sheet.cur_x, sheet.cur_y, sheet.cur_z);
            } else {
                state.status_message = String::from("No block marked. Press m twice to mark a block.");
            }
        }
        "ir" | "insert-row" => {
            sheet.save_undo();
            sheet.insert_row(sheet.cur_y, sheet.cur_z);
            sheet.update();
            state.status_message = format!("Inserted row at {}", sheet.cur_y);
        }
        "dr" | "delete-row" => {
            sheet.save_undo();
            sheet.delete_row(sheet.cur_y, sheet.cur_z);
            sheet.update();
            state.status_message = format!("Deleted row {}", sheet.cur_y);
        }
        "ic" | "insert-col" => {
            sheet.save_undo();
            sheet.insert_col(sheet.cur_x, sheet.cur_z);
            sheet.update();
            state.status_message = format!("Inserted column at {}", sheet.cur_x);
        }
        "dc" | "delete-col" => {
            sheet.save_undo();
            sheet.delete_col(sheet.cur_x, sheet.cur_z);
            sheet.update();
            state.status_message = format!("Deleted column {}", sheet.cur_x);
        }
        "sort" | "sort-x" => {
            if let Some((x1, y1, z1, x2, y2, z2)) = sheet.get_mark_range() {
                let parts: Vec<&str> = arg.split_whitespace().collect();
                let sort_col = parts.first()
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(sheet.cur_x);
                let ascending = parts.get(1)
                    .map(|s| !s.starts_with('d'))
                    .unwrap_or(true);
                sheet.save_undo();
                sheet.sort_block(x1, y1, z1, x2, y2, z2, sort_col, ascending);
                let dir = if ascending { "ascending" } else { "descending" };
                state.status_message = format!("Sorted by column {} {}", sort_col, dir);
            } else {
                state.status_message = String::from("No block marked. Press m twice to mark a block.");
            }
        }
        "sort-y" => {
            if let Some((x1, y1, z1, x2, y2, z2)) = sheet.get_mark_range() {
                let parts: Vec<&str> = arg.split_whitespace().collect();
                let sort_row = parts.first()
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(sheet.cur_y);
                let ascending = parts.get(1)
                    .map(|s| !s.starts_with('d'))
                    .unwrap_or(true);
                sheet.save_undo();
                sheet.sort_block_y(x1, y1, z1, x2, y2, z2, sort_row, ascending);
                let dir = if ascending { "ascending" } else { "descending" };
                state.status_message = format!("Sorted by row {} {}", sort_row, dir);
            } else {
                state.status_message = String::from("No block marked. Press m twice to mark a block.");
            }
        }
        "sort-z" => {
            if let Some((x1, y1, z1, x2, y2, z2)) = sheet.get_mark_range() {
                let parts: Vec<&str> = arg.split_whitespace().collect();
                // sort-z expects x,y coords for the sort key
                let (sort_x, sort_y) = if let Some(coord) = parts.first() {
                    let coords: Vec<&str> = coord.split(',').collect();
                    (
                        coords.first().and_then(|s| s.parse().ok()).unwrap_or(sheet.cur_x),
                        coords.get(1).and_then(|s| s.parse().ok()).unwrap_or(sheet.cur_y),
                    )
                } else {
                    (sheet.cur_x, sheet.cur_y)
                };
                let ascending = parts.get(1)
                    .map(|s| !s.starts_with('d'))
                    .unwrap_or(true);
                sheet.save_undo();
                sheet.sort_block_z(x1, y1, z1, x2, y2, z2, sort_x, sort_y, ascending);
                let dir = if ascending { "ascending" } else { "descending" };
                state.status_message = format!("Sorted layers by ({},{}) {}", sort_x, sort_y, dir);
            } else {
                state.status_message = String::from("No block marked. Press m twice to mark a block.");
            }
        }
        "mirror-x" => {
            if let Some((x1, y1, z1, x2, y2, z2)) = sheet.get_mark_range() {
                sheet.save_undo();
                sheet.mirror_block(x1, y1, z1, x2, y2, z2, crate::sheet::Direction::X);
                sheet.clear_mark();
                state.status_message = String::from("Mirrored block horizontally");
            } else {
                state.status_message = String::from("No block marked. Press m twice to mark a block.");
            }
        }
        "mirror-y" => {
            if let Some((x1, y1, z1, x2, y2, z2)) = sheet.get_mark_range() {
                sheet.save_undo();
                sheet.mirror_block(x1, y1, z1, x2, y2, z2, crate::sheet::Direction::Y);
                sheet.clear_mark();
                state.status_message = String::from("Mirrored block vertically");
            } else {
                state.status_message = String::from("No block marked. Press m twice to mark a block.");
            }
        }
        "mirror-z" => {
            if let Some((x1, y1, z1, x2, y2, z2)) = sheet.get_mark_range() {
                sheet.save_undo();
                sheet.mirror_block(x1, y1, z1, x2, y2, z2, crate::sheet::Direction::Z);
                sheet.clear_mark();
                state.status_message = String::from("Mirrored block across layers");
            } else {
                state.status_message = String::from("No block marked. Press m twice to mark a block.");
            }
        }
        "fill" => {
            if let Some((x1, y1, z1, x2, y2, z2)) = sheet.get_mark_range() {
                let parts: Vec<&str> = arg.split_whitespace().collect();
                let cols = parts.first().and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
                let rows = parts.get(1).and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
                let layers = parts.get(2).and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
                if cols == 0 || rows == 0 || layers == 0 {
                    state.status_message = String::from("Usage: :fill cols rows [layers] (all > 0)");
                } else {
                    sheet.save_undo();
                    let count = sheet.fill_block(x1, y1, z1, x2, y2, z2, cols, rows, layers);
                    sheet.clear_mark();
                    state.status_message = format!("Filled {} cells ({}x{}x{} grid)", count, cols, rows, layers);
                }
            } else {
                state.status_message = String::from("No block marked. Press m twice to mark a block.");
            }
        }
        "move" => {
            if let Some((x1, y1, z1, x2, y2, z2)) = sheet.get_mark_range() {
                sheet.save_undo();
                let count = sheet.move_block(x1, y1, z1, x2, y2, z2,
                    sheet.cur_x, sheet.cur_y, sheet.cur_z);
                sheet.clear_mark();
                state.status_message = format!("Moved {} cells to ({},{},{})",
                    count, sheet.cur_x, sheet.cur_y, sheet.cur_z);
            } else {
                state.status_message = String::from("No block marked. Press m twice to mark a block.");
            }
        }
        "undo" => {
            if sheet.undo() {
                state.status_message = String::from("Undone");
            } else {
                state.status_message = String::from("Nothing to undo");
            }
        }
        "redo" => {
            if sheet.redo() {
                state.status_message = String::from("Redone");
            } else {
                state.status_message = String::from("Nothing to redo");
            }
        }
        "yank" => {
            let count = sheet.yank_block();
            if count > 0 {
                state.status_message = format!("Yanked {} cells", count);
            } else {
                state.status_message = String::from("No block marked to yank");
            }
        }
        "paste" => {
            if sheet.clipboard.is_empty() {
                state.status_message = String::from("Clipboard empty");
            } else {
                let count = sheet.paste();
                state.status_message = format!("Pasted {} cells", count);
            }
        }
        "export-text" | "save-text" => {
            if arg.is_empty() {
                state.status_message = String::from("Usage: :export-text <file>");
            } else {
                let x2 = sheet.dim_x.saturating_sub(1);
                let y2 = sheet.dim_y.saturating_sub(1);
                match crate::fileio::save_text(sheet, arg, 0, 0, 0, x2, y2, 0) {
                    Ok(count) => {
                        state.status_message = format!("Exported {} cells to {}", count, arg);
                    }
                    Err(e) => {
                        state.status_message = format!("Export failed: {}", e);
                    }
                }
            }
        }
        // Sheet navigation commands
        "sheet" => {
            if let Ok(n) = arg.parse::<usize>() {
                if n > 0 {
                    sheet.cur_z = n - 1;
                    if sheet.cur_z >= sheet.dim_z {
                        sheet.dim_z = sheet.cur_z + 1;
                    }
                    state.status_message = format!("Switched to Sheet {}", n);
                } else {
                    state.status_message = String::from("Sheet number must be > 0");
                }
            } else {
                state.status_message = format!("Current: Sheet {} of {}", sheet.cur_z + 1, sheet.dim_z);
            }
        }
        "sheet-add" => {
            sheet.dim_z += 1;
            sheet.cur_z = sheet.dim_z - 1;
            sheet.changed = true;
            state.status_message = format!("Added Sheet {} (total: {})", sheet.dim_z, sheet.dim_z);
        }
        "sheet-del" => {
            if sheet.dim_z > 1 {
                // Clear all cells on the current sheet layer
                let z = sheet.cur_z;
                let keys: Vec<_> = sheet.cell_coords().into_iter()
                    .filter(|&(_, _, cz)| cz == z)
                    .collect();
                for (x, y, z) in keys {
                    sheet.clear_block(x, y, z, x, y, z);
                }
                // Shift sheets above down (done implicitly by dim_z reduction)
                if sheet.cur_z >= sheet.dim_z - 1 {
                    sheet.cur_z = sheet.dim_z - 2;
                }
                sheet.dim_z -= 1;
                sheet.changed = true;
                state.status_message = format!("Deleted sheet (remaining: {})", sheet.dim_z);
            } else {
                state.status_message = String::from("Cannot delete the last sheet");
            }
        }
        "sheets" => {
            state.sheet_picker_active = true;
            state.sheet_picker_selection = sheet.cur_z;
        }
        // Clock commands
        "clock" => {
            let (x, y, z) = (sheet.cur_x, sheet.cur_y, sheet.cur_z);
            let enabled = sheet.toggle_clock(x, y, z);
            state.status_message = format!("Clock {}", if enabled { "enabled" } else { "disabled" });
        }
        "clock-run" => {
            let mut total = 0;
            for _ in 0..1000 {
                let count = sheet.clock_tick();
                if count == 0 { break; }
                total += count;
            }
            state.status_message = format!("Clock run: {} total updates", total);
        }
        "search" | "s" => {
            if arg.is_empty() {
                // Open interactive search
                state.search_active = true;
                state.search_field = SearchField::Search;
                state.replace_active = false;
                state.search_pattern.clear();
                state.search_results.clear();
                state.search_index = 0;
                state.search_all_layers = false;
            } else {
                state.search_pattern = arg.to_string();
                state.search_in_values = true;
                state.search_all_layers = false;
                update_search_results(sheet, state);
                if !state.search_results.is_empty() {
                    state.search_index = 0;
                    let (x, y, z) = state.search_results[0];
                    sheet.cur_x = x;
                    sheet.cur_y = y;
                    sheet.cur_z = z;
                    state.status_message = format!("{} matches. n/N to navigate.", state.search_results.len());
                } else {
                    state.status_message = format!("No matches for '{}'", arg);
                }
            }
        }
        "search-all" => {
            if arg.is_empty() {
                state.search_active = true;
                state.search_field = SearchField::Search;
                state.replace_active = false;
                state.search_pattern.clear();
                state.search_results.clear();
                state.search_index = 0;
                state.search_all_layers = true;
            } else {
                state.search_pattern = arg.to_string();
                state.search_in_values = true;
                state.search_all_layers = true;
                update_search_results(sheet, state);
                if !state.search_results.is_empty() {
                    state.search_index = 0;
                    let (x, y, z) = state.search_results[0];
                    sheet.cur_x = x;
                    sheet.cur_y = y;
                    sheet.cur_z = z;
                    state.status_message = format!("{} matches across all sheets. n/N to navigate.", state.search_results.len());
                } else {
                    state.status_message = format!("No matches for '{}'", arg);
                }
            }
        }
        "search-formula" => {
            if arg.is_empty() {
                state.search_active = true;
                state.search_field = SearchField::Search;
                state.replace_active = false;
                state.search_pattern.clear();
                state.search_results.clear();
                state.search_index = 0;
                state.search_in_values = false;
                state.search_all_layers = false;
            } else {
                state.search_pattern = arg.to_string();
                state.search_in_values = false;
                state.search_all_layers = false;
                update_search_results(sheet, state);
                if !state.search_results.is_empty() {
                    state.search_index = 0;
                    let (x, y, z) = state.search_results[0];
                    sheet.cur_x = x;
                    sheet.cur_y = y;
                    sheet.cur_z = z;
                    state.status_message = format!("{} formula matches. n/N to navigate.", state.search_results.len());
                } else {
                    state.status_message = format!("No formula matches for '{}'", arg);
                }
            }
        }
        "replace" | "r" => {
            // :replace <search> <replace> or :r <search> <replace>
            let rparts: Vec<&str> = arg.splitn(2, ' ').collect();
            if rparts.len() == 2 {
                state.search_pattern = rparts[0].to_string();
                state.replace_pattern = rparts[1].to_string();
                state.search_in_values = true;
                state.search_all_layers = false;
                update_search_results(sheet, state);
                if !state.search_results.is_empty() {
                    state.search_index = 0;
                    let (x, y, z) = state.search_results[0];
                    sheet.cur_x = x;
                    sheet.cur_y = y;
                    sheet.cur_z = z;
                    state.replace_confirm = true;
                    state.status_message = format!(
                        "Replace? (y)es (n)o (a)ll (q)uit — Match 1/{}",
                        state.search_results.len()
                    );
                } else {
                    state.status_message = format!("No matches for '{}'", rparts[0]);
                }
            } else if arg.is_empty() {
                // Open interactive search+replace
                state.search_active = true;
                state.search_field = SearchField::Search;
                state.replace_active = true;
                state.search_pattern.clear();
                state.replace_pattern.clear();
                state.search_results.clear();
                state.search_index = 0;
                state.search_all_layers = false;
            } else {
                state.status_message = String::from("Usage: :replace <search> <replacement>");
            }
        }
        "replace-all" => {
            let rparts: Vec<&str> = arg.splitn(2, ' ').collect();
            if rparts.len() == 2 {
                state.search_pattern = rparts[0].to_string();
                state.replace_pattern = rparts[1].to_string();
                state.search_in_values = true;
                state.search_all_layers = false;
                update_search_results(sheet, state);
                if !state.search_results.is_empty() {
                    let re = if state.search_regex {
                        Regex::new(&state.search_pattern).unwrap_or_else(|_| Regex::new(&regex::escape(&state.search_pattern)).unwrap())
                    } else {
                        Regex::new(&regex::escape(&state.search_pattern)).unwrap()
                    };
                    sheet.save_undo();
                    let mut count = 0;
                    for &(x, y, z) in &state.search_results {
                        if sheet.replace_cell(x, y, z, &re, &state.replace_pattern, state.search_in_values) {
                            count += 1;
                        }
                    }
                    sheet.update();
                    state.search_results.clear();
                    state.status_message = format!("Replaced {} matches", count);
                } else {
                    state.status_message = format!("No matches for '{}'", rparts[0]);
                }
            } else {
                state.status_message = String::from("Usage: :replace-all <search> <replacement>");
            }
        }
        "help" => {
            state.input_mode = InputMode::Help;
        }
        _ => {
            state.status_message = format!("Unknown command: {}", cmd_owned);
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
    if state.input_mode == InputMode::Help {
        render_help(f, chunks[1]);
    } else {
        render_sheet(f, sheet, state, chunks[1]);
    }

    // Status bar: show cell info with colored cell references
    // When cycling refs with g, show the origin cell's formula instead
    let (status_cell_x, status_cell_y, status_cell_z) = state.goto_ref_origin
        .unwrap_or((sheet.cur_x, sheet.cur_y, sheet.cur_z));
    let coord_str = format!("({},{},{}) ", sheet.cur_x, sheet.cur_y, sheet.cur_z);
    let base_style = Style::default().fg(Color::White).bg(Color::Blue);
    let status_spans = if let Some(cell) = sheet.get_cell(status_cell_x, status_cell_y, status_cell_z) {
        if let Some(ref contents) = cell.contents {
            let refs = extract_cell_refs(contents);
            let formula = scanner::print_tokens(contents, true, cell.scientific, cell.precision);
            let mut spans = vec![Span::styled(coord_str, base_style)];
            // Color cell references in the formula string
            let mut remaining = formula.as_str();
            for (ri, &(rx, ry, rz)) in refs.iter().enumerate() {
                let ref_str = format!("@({},{},{})", rx, ry, rz);
                if let Some(pos) = remaining.find(&ref_str) {
                    if pos > 0 {
                        spans.push(Span::styled(remaining[..pos].to_string(), base_style));
                    }
                    let color = REF_COLORS[ri % REF_COLORS.len()];
                    spans.push(Span::styled(
                        ref_str.clone(),
                        Style::default().fg(color).bg(Color::Blue).add_modifier(Modifier::BOLD),
                    ));
                    remaining = &remaining[pos + ref_str.len()..];
                } else {
                    // Reference not found literally (e.g. uses expressions), skip coloring
                }
            }
            if !remaining.is_empty() {
                spans.push(Span::styled(remaining.to_string(), base_style));
            }
            spans.push(Span::styled(format!(" | {}", state.status_message), base_style));
            spans
        } else {
            vec![
                Span::styled(coord_str, base_style),
                Span::styled(format!("| {}", state.status_message), base_style),
            ]
        }
    } else {
        vec![
            Span::styled(coord_str, base_style),
            Span::styled(format!("| {}", state.status_message), base_style),
        ]
    };
    let status = Paragraph::new(Line::from(status_spans))
        .style(base_style);
    f.render_widget(status, chunks[2]);

    // Input bar
    if state.search_active {
        // Search bar replaces input line
        let mode = if state.search_in_values { "val" } else { "formula" };
        let re_mode = if state.search_regex { "re" } else { "lit" };
        let count = state.search_results.len();
        if state.replace_active {
            let (search_style, replace_style) = if state.search_field == SearchField::Search {
                (Style::default().fg(Color::Yellow), Style::default().fg(Color::White))
            } else {
                (Style::default().fg(Color::White), Style::default().fg(Color::Yellow))
            };
            let line = Line::from(vec![
                Span::styled(format!("/{} ", re_mode), Style::default().fg(Color::DarkGray)),
                Span::styled(format!("search: {} ", state.search_pattern), search_style),
                Span::styled(format!("→ replace: {} ", state.replace_pattern), replace_style),
                Span::styled(format!("[{}] {}", mode, count), Style::default().fg(Color::DarkGray)),
            ]);
            let bar = Paragraph::new(line);
            f.render_widget(bar, chunks[3]);
            // Set cursor position
            if state.search_field == SearchField::Search {
                let prefix_len = re_mode.len() + 2 + "search: ".len();
                f.set_cursor_position((
                    chunks[3].x + prefix_len as u16 + state.search_pattern.len() as u16,
                    chunks[3].y,
                ));
            } else {
                let prefix_len = re_mode.len() + 2 + "search: ".len() + state.search_pattern.len() + " → replace: ".len();
                f.set_cursor_position((
                    chunks[3].x + prefix_len as u16 + state.replace_pattern.len() as u16,
                    chunks[3].y,
                ));
            }
        } else {
            let line = Line::from(vec![
                Span::styled(format!("/{} ", re_mode), Style::default().fg(Color::DarkGray)),
                Span::styled(format!("search: {}", state.search_pattern), Style::default().fg(Color::Yellow)),
                Span::styled(format!("  [{}] {}", mode, count), Style::default().fg(Color::DarkGray)),
            ]);
            let bar = Paragraph::new(line);
            f.render_widget(bar, chunks[3]);
            let prefix_len = re_mode.len() + 2 + "search: ".len();
            f.set_cursor_position((
                chunks[3].x + prefix_len as u16 + state.search_pattern.len() as u16,
                chunks[3].y,
            ));
        }
    } else if state.replace_confirm {
        let msg = &state.status_message;
        let bar = Paragraph::new(msg.as_str())
            .style(Style::default().fg(Color::Yellow).bg(Color::DarkGray));
        f.render_widget(bar, chunks[3]);
    } else {
    match state.input_mode {
        InputMode::Normal => {
            let help_text = if state.palette_active || state.sheet_picker_active || state.picker_active {
                " Use arrow keys to navigate, Enter to select, Esc to cancel"
            } else {
                " hjkl: move | =/': formula/text | 0-9: number | :: command | /: palette | n: search | ?: help"
            };
            let help = Paragraph::new(help_text)
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(help, chunks[3]);
        }
        InputMode::Editing => {
            let prefix = if state.text_editing { "Text: " } else { "Edit: " };
            let display = format!("{}{}", prefix, state.input_buffer);
            let input = Paragraph::new(display)
                .style(Style::default().fg(Color::Yellow));
            f.render_widget(input, chunks[3]);
            f.set_cursor_position((
                chunks[3].x + prefix.len() as u16 + state.cursor_position as u16,
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

    // Render overlays
    if state.palette_active {
        render_palette(f, state);
    }
    if state.sheet_picker_active {
        render_sheet_picker(f, sheet, state);
    }
    if state.picker_active {
        render_cell_picker_info(f, state);
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
    Home          Go to (0,0)         End Go to last cell
    PgUp/PgDn     Page up/down        Tab Next sheet
    [ / ]         Prev/next sheet     Z   Sheet picker
    +/-           Widen/narrow column

  Editing
    = / Enter     Formula / edit cell  '   Text entry (auto-quotes)
    0-9           Quick number entry   m   Mark block (twice)
    Delete        Clear current cell   u   Clear mark
    y             Yank (copy) block    p   Paste at cursor
    Ctrl+Z        Undo                 Ctrl+Y  Redo
    Esc           Cancel / clear search  C  Clock tick
    n             Search / next match  N   Previous match
    r             Search and replace   g   Go to cell reference (cycle)
    d             Go to dependent cell (cycle)

  Edit mode keys
    Left/Right    Move cursor          Home/End  Jump to start/end
    Ctrl+A/E      Beginning/end        Ctrl+K    Kill to end
    Ctrl+U        Kill to beginning    Ctrl+W    Delete word back
    @             Cell picker (select cell reference)

  Commands (: for command mode, / or F1 for palette)
    :w [file]     Save (.tp/.tpz/.xlsx)  :o <file>  Open file
    :q            Quit                  :q!        Force quit
    :wq           Save and quit         :goto x,y  Move to cell
    :width N      Set column width      :align l/r/c/a
    :precision N  Set decimal places    :bold  :underline
    :ir/:dr       Insert/delete row     :ic/:dc    Insert/delete col
    :yank         Yank block to clip    :paste     Paste at cursor
    :undo         Undo last change      :redo      Redo last undo
    :copy         Copy block to cursor  :move      Move block to cursor
    :clear        Clear marked block    :sort [col] [asc|desc]
    :sort-y       Sort cols by row      :sort-z    Sort layers
    :mirror-x/y/z Mirror block          :fill c r [l]  Tile block
    :sheet N      Switch to sheet N     :sheet-add/:sheet-del
    :clock        Toggle cell clock     :clock-run Run clock
    :search <pat> Search current sheet  :s <pat>   Alias for search
    :search-all   Search all sheets     :search-formula  Search formulas
    :replace <s> <r>  Replace with confirmation  :replace-all <s> <r>
    :export-text  Export as plain text  :export-csv   Export as CSV
    :export-html  Export as HTML        :export-latex Export as LaTeX
    :export-context  Export as ConTeXt  :help         Show this help

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

/// A palette of distinct colors for highlighting cell references
const REF_COLORS: &[Color] = &[
    Color::LightBlue,
    Color::LightMagenta,
    Color::LightGreen,
    Color::LightRed,
    Color::LightCyan,
    Color::Rgb(255, 165, 0), // orange
];

/// Extract static cell references from a token list.
/// Returns coordinates for `@(x,y,z)` calls with literal integer args.
fn extract_cell_refs(tokens: &[Token]) -> Vec<(usize, usize, usize)> {
    let mut refs = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if let Token::Identifier(name) = &tokens[i] {
            if name == "@"
                && i + 7 < tokens.len()
                && tokens[i + 1] == Token::Operator(Operator::OpenParen)
                && tokens[i + 3] == Token::Operator(Operator::Comma)
                && tokens[i + 5] == Token::Operator(Operator::Comma)
                && tokens[i + 7] == Token::Operator(Operator::CloseParen)
            {
                if let (Some(x), Some(y), Some(z)) = (
                    token_to_usize(&tokens[i + 2]),
                    token_to_usize(&tokens[i + 4]),
                    token_to_usize(&tokens[i + 6]),
                ) {
                    refs.push((x, y, z));
                    i += 8;
                    continue;
                }
            }
        }
        i += 1;
    }
    refs
}

/// Try to extract a usize from an Integer or Float token
fn token_to_usize(token: &Token) -> Option<usize> {
    match token {
        Token::Integer(n) => {
            if *n >= 0 { Some(*n as usize) } else { None }
        }
        Token::Float(f) => {
            if *f >= 0.0 { Some(*f as usize) } else { None }
        }
        _ => None,
    }
}

/// Find all cells that reference a given cell (dependents/reverse references).
fn find_dependents(sheet: &Sheet, tx: usize, ty: usize, tz: usize) -> Vec<(usize, usize, usize)> {
    let mut deps = Vec::new();
    for (&(cx, cy, cz), cell) in sheet.cells() {
        if let Some(ref contents) = cell.contents {
            let refs = extract_cell_refs(contents);
            if refs.contains(&(tx, ty, tz)) {
                deps.push((cx, cy, cz));
            }
        }
    }
    deps.sort_by(|a, b| a.2.cmp(&b.2).then(a.1.cmp(&b.1)).then(a.0.cmp(&b.0)));
    deps
}

fn render_sheet(f: &mut Frame, sheet: &Sheet, state: &DisplayState, area: Rect) {
    if area.height < 3 || area.width < 6 {
        return;
    }

    let z = sheet.cur_z;

    // Calculate visible columns based on actual column widths
    let row_num_width = 4u16;
    let mut col_widths: Vec<(usize, u16)> = Vec::new(); // (col_index, width)
    let mut used_width = row_num_width;

    let start_col = sheet.off_x;
    for x in start_col.. {
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

    // Extract cell references: use goto_ref origin if cycling, else current cell
    let (ref_source_x, ref_source_y, ref_source_z) = state.goto_ref_origin
        .unwrap_or((sheet.cur_x, sheet.cur_y, sheet.cur_z));
    let cur_refs: Vec<(usize, usize, usize)> = sheet
        .get_cell(ref_source_x, ref_source_y, ref_source_z)
        .and_then(|c| c.contents.as_ref())
        .map(|tokens| extract_cell_refs(tokens))
        .unwrap_or_default();

    // Dependents: always show for focused cell; use d-cycle origin if active
    let dep_source = state.goto_dep_origin
        .unwrap_or((sheet.cur_x, sheet.cur_y, sheet.cur_z));
    let cur_deps: Vec<(usize, usize, usize)> = find_dependents(sheet, dep_source.0, dep_source.1, dep_source.2);

    // Build data rows
    let mut rows = Vec::new();
    for dy in 0..visible_rows {
        let y = start_row + dy;
        let mut cells = vec![
            TuiCell::from(format!("{}", y))
                .style(Style::default().fg(Color::Yellow))
        ];

        for &(x, _) in &col_widths {
            let cell_data = sheet.get_cell(x, y, z);
            let content = if let Some(cell) = cell_data {
                cell.value.to_string()
            } else {
                String::new()
            };

            let has_formula = cell_data
                .and_then(|c| c.contents.as_ref())
                .is_some_and(|tokens| tokens.len() > 1 || matches!(tokens.first(), Some(Token::Identifier(_))));

            let in_mark = sheet.get_mark_range().is_some_and(|(x1, y1, z1, x2, y2, z2)|
                x >= x1 && x <= x2 && y >= y1 && y <= y2 && z >= z1 && z <= z2);
            let is_picker = state.picker_active && x == state.picker_x && y == state.picker_y && z == state.picker_z;
            let search_match_idx = state.search_results.iter().position(|&(sx, sy, sz)| sx == x && sy == y && sz == z);
            let is_current_match = search_match_idx == Some(state.search_index) && !state.search_results.is_empty();
            let is_any_match = search_match_idx.is_some();
            // Check if this cell is referenced by the current cell's formula
            let ref_index = cur_refs.iter().position(|&(rx, ry, rz)| rx == x && ry == y && rz == z);
            let dep_index = cur_deps.iter().position(|&(dx, dy, dz)| dx == x && dy == y && dz == z);

            let style = if is_picker {
                Style::default().fg(Color::Black).bg(Color::Green)
            } else if x == sheet.cur_x && y == sheet.cur_y {
                if is_current_match {
                    Style::default().fg(Color::Black).bg(Color::LightYellow)
                } else {
                    Style::default().fg(Color::Black).bg(Color::White)
                }
            } else if let Some(ri) = ref_index {
                let color = REF_COLORS[ri % REF_COLORS.len()];
                Style::default().fg(Color::Black).bg(color)
            } else if let Some(di) = dep_index {
                let color = REF_COLORS[di % REF_COLORS.len()];
                Style::default().fg(color).bg(Color::DarkGray)
            } else if is_current_match {
                Style::default().fg(Color::Black).bg(Color::LightYellow)
            } else if is_any_match {
                Style::default().fg(Color::Black).bg(Color::Yellow)
            } else if in_mark {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else if has_formula {
                Style::default().bg(Color::Rgb(20, 30, 40))
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

/// Render the command palette popup
fn render_palette(f: &mut Frame, state: &DisplayState) {
    let area = f.area();
    // Center the palette
    let width = 40u16.min(area.width.saturating_sub(4));
    let height = 20u16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect::new(x, y, width, height);

    f.render_widget(Clear, popup_area);

    let filtered = get_palette_items(&state.palette_filter);

    let items: Vec<ListItem> = filtered.iter()
        .enumerate()
        .take(height.saturating_sub(3) as usize)
        .map(|(i, cmd)| {
            let style = if i == state.palette_selection {
                Style::default().fg(Color::Black).bg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(Span::styled(format!(":{}", cmd), style)))
        })
        .collect();

    let title = format!("Commands [{}]", if state.palette_filter.is_empty() { "*" } else { &state.palette_filter });
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title).style(Style::default().bg(Color::DarkGray)));

    f.render_widget(list, popup_area);
}

/// Render the sheet picker popup
fn render_sheet_picker(f: &mut Frame, sheet: &Sheet, state: &DisplayState) {
    let area = f.area();
    let max_z = sheet.dim_z.max(1);
    let height = (max_z as u16 + 2).min(area.height.saturating_sub(4));
    let width = 25u16.min(area.width.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect::new(x, y, width, height);

    f.render_widget(Clear, popup_area);

    let items: Vec<ListItem> = (0..max_z)
        .map(|i| {
            let style = if i == state.sheet_picker_selection {
                Style::default().fg(Color::Black).bg(Color::Yellow)
            } else if i == sheet.cur_z {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };
            let marker = if i == sheet.cur_z { " *" } else { "" };
            ListItem::new(Line::from(Span::styled(format!(" Sheet {}{}", i + 1, marker), style)))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Sheets").style(Style::default().bg(Color::DarkGray)));

    f.render_widget(list, popup_area);
}

/// Render cell picker info bar
fn render_cell_picker_info(f: &mut Frame, state: &DisplayState) {
    let area = f.area();
    if area.height < 3 { return; }
    // Show a small info bar at the top
    let info = format!(" Cell picker: @({},{},{})  [arrows to move, Enter to insert, Esc to cancel]",
        state.picker_x, state.picker_y, state.picker_z);
    let popup_area = Rect::new(0, 0, area.width, 1);
    f.render_widget(Clear, popup_area);
    let widget = Paragraph::new(info)
        .block(Block::default().style(Style::default().bg(Color::Green)))
        .style(Style::default().fg(Color::Black).bg(Color::Green));
    f.render_widget(widget, popup_area);
}
