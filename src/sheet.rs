//! Sheet module - defines the spreadsheet data structure

use crate::token::Token;
use regex::Regex;
use std::cmp::Ordering;
use std::collections::HashMap;

/// Compare two Token values for sorting: Empty < numbers < strings
fn cmp_token_values(a: Option<&Token>, b: Option<&Token>) -> Ordering {
    match (a, b) {
        (None, None) | (Some(Token::Empty), Some(Token::Empty)) => Ordering::Equal,
        (None | Some(Token::Empty), _) => Ordering::Less,
        (_, None | Some(Token::Empty)) => Ordering::Greater,
        (Some(Token::Integer(i)), Some(Token::Integer(j))) => i.cmp(j),
        (Some(Token::Float(f1)), Some(Token::Float(f2))) => f1.partial_cmp(f2).unwrap_or(Ordering::Equal),
        (Some(Token::Integer(i)), Some(Token::Float(f))) => (*i as f64).partial_cmp(f).unwrap_or(Ordering::Equal),
        (Some(Token::Float(f)), Some(Token::Integer(i))) => f.partial_cmp(&(*i as f64)).unwrap_or(Ordering::Equal),
        (Some(Token::String(s1)), Some(Token::String(s2))) => s1.cmp(s2),
        (Some(Token::Integer(_) | Token::Float(_)), Some(Token::String(_))) => Ordering::Less,
        (Some(Token::String(_)), Some(Token::Integer(_) | Token::Float(_))) => Ordering::Greater,
        _ => Ordering::Equal,
    }
}

/// Cell represents a single cell in the spreadsheet
#[derive(Debug, Clone)]
pub struct Cell {
    /// The raw contents (formula or value)
    pub contents: Option<Vec<Token>>,
    /// The calculated contents (for clocked cells)
    pub clocked_contents: Option<Vec<Token>>,
    /// The cell's label
    pub label: Option<String>,
    /// The calculated value
    pub value: Token,
    /// The calculated value for clocked cells
    pub clocked_value: Token,
    /// Text alignment
    pub adjust: Adjust,
    /// Decimal precision
    pub precision: i32,
    /// Whether the cell has been updated
    pub updated: bool,
    /// Whether the cell is shadowed
    pub shadowed: bool,
    /// Whether to use scientific notation
    pub scientific: bool,
    /// Whether the cell is locked
    pub locked: bool,
    /// Whether the cell is transparent
    pub transparent: bool,
    /// Whether the cell is ignored in calculations
    pub ignored: bool,
    /// Clock flags
    pub clock_t0: bool,
    pub clock_t1: bool,
    pub clock_t2: bool,
    /// Whether the cell is bold
    pub bold: bool,
    /// Whether the cell is underlined
    pub underline: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            contents: None,
            clocked_contents: None,
            label: None,
            value: Token::Empty,
            clocked_value: Token::Empty,
            adjust: Adjust::AutoAdjust,
            precision: -1,
            updated: false,
            shadowed: false,
            scientific: false,
            locked: false,
            transparent: false,
            ignored: false,
            clock_t0: false,
            clock_t1: false,
            clock_t2: false,
            bold: false,
            underline: false,
        }
    }
}

/// Text alignment options
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Adjust {
    /// Left-aligned text
    Left,
    /// Right-aligned text
    Right,
    /// Center-aligned text
    Center,
    /// Auto-adjusted based on content
    AutoAdjust,
}

/// Direction for operations like insert, delete, sort
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    /// X direction (columns)
    X,
    /// Y direction (rows)
    Y,
    /// Z direction (sheets)
    Z,
}

/// Sort key for sorting operations
#[derive(Debug, Clone)]
pub struct SortKey {
    /// X position
    pub x: usize,
    /// Y position
    pub y: usize,
    /// Z position
    pub z: usize,
    /// Sort flags
    pub sort_key: u32,
}

/// Snapshot of sheet data for undo
#[derive(Debug, Clone)]
struct UndoSnapshot {
    cells: HashMap<(usize, usize, usize), Cell>,
    column_widths: HashMap<(usize, usize), usize>,
    dim_x: usize,
    dim_y: usize,
    dim_z: usize,
}

/// Sheet represents the entire spreadsheet
#[derive(Debug)]
pub struct Sheet {
    /// The cells in the sheet
    cells: HashMap<(usize, usize, usize), Cell>,
    /// Column widths
    column_widths: HashMap<(usize, usize), usize>,
    /// Current cursor position
    pub cur_x: usize,
    pub cur_y: usize,
    pub cur_z: usize,
    /// Mark positions for block operations
    pub mark1_x: Option<usize>,
    pub mark1_y: Option<usize>,
    pub mark1_z: Option<usize>,
    pub mark2_x: Option<usize>,
    pub mark2_y: Option<usize>,
    pub mark2_z: Option<usize>,
    /// Whether marking is in progress
    pub marking: bool,
    /// Display offset
    pub off_x: usize,
    pub off_y: usize,
    /// Sheet dimensions
    pub dim_x: usize,
    pub dim_y: usize,
    pub dim_z: usize,
    /// Origin coordinates
    pub ori_x: usize,
    pub ori_y: usize,
    /// Maximum display dimensions
    pub max_x: usize,
    pub max_y: usize,
    /// Default column width
    pub width: usize,
    /// Sheet name
    pub name: Option<String>,
    /// Whether the sheet has been modified
    pub changed: bool,
    /// Whether only movement is allowed
    pub move_only: bool,
    /// Whether clock is enabled
    pub clk: bool,
    /// Label cache for quick lookups
    label_cache: HashMap<String, (usize, usize, usize)>,
    /// Undo stack (snapshots of cell data)
    undo_stack: Vec<UndoSnapshot>,
    /// Redo stack (snapshots popped from undo)
    redo_stack: Vec<UndoSnapshot>,
    /// Clipboard: copied cells with relative positions
    pub clipboard: Vec<((usize, usize, usize), Cell)>,
}

impl Sheet {
    /// Create a new empty sheet
    pub fn new() -> Self {
        Sheet {
            cells: HashMap::new(),
            column_widths: HashMap::new(),
            cur_x: 0,
            cur_y: 0,
            cur_z: 0,
            mark1_x: None,
            mark1_y: None,
            mark1_z: None,
            mark2_x: None,
            mark2_y: None,
            mark2_z: None,
            marking: false,
            off_x: 0,
            off_y: 0,
            dim_x: 10,
            dim_y: 10,
            dim_z: 1,
            ori_x: 0,
            ori_y: 0,
            max_x: 80,
            max_y: 24,
            width: 10,
            name: None,
            changed: false,
            move_only: false,
            clk: false,
            label_cache: HashMap::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            clipboard: Vec::new(),
        }
    }

    /// Get a cell at the specified coordinates
    pub fn get_cell(&self, x: usize, y: usize, z: usize) -> Option<&Cell> {
        self.cells.get(&(x, y, z))
    }

    /// Get a mutable reference to a cell at the specified coordinates
    pub fn get_cell_mut(&mut self, x: usize, y: usize, z: usize) -> Option<&mut Cell> {
        self.cells.get_mut(&(x, y, z))
    }

    /// Get or create a cell at the specified coordinates
    pub fn get_or_create_cell(&mut self, x: usize, y: usize, z: usize) -> &mut Cell {
        self.cells.entry((x, y, z)).or_insert_with(Cell::default)
    }

    /// Get the width of a column
    pub fn column_width(&self, x: usize, z: usize) -> usize {
        self.column_widths.get(&(x, z)).copied().unwrap_or(self.width)
    }

    /// Set the width of a column
    pub fn set_width(&mut self, x: usize, z: usize, width: usize) {
        self.column_widths.insert((x, z), width);
    }

    /// Resize the sheet
    pub fn resize(&mut self, x: usize, y: usize, z: usize) {
        self.dim_x = x;
        self.dim_y = y;
        self.dim_z = z;
    }

    /// Look up a label in the cache and return the cell's value
    pub fn findlabel(&self, label: &str) -> Token {
        if let Some(&(x, y, z)) = self.label_cache.get(label) {
            if let Some(cell) = self.get_cell(x, y, z) {
                cell.value.clone()
            } else {
                Token::Empty
            }
        } else {
            Token::Error(format!("label '{}' not found", label))
        }
    }

    /// Rebuild the label cache from all cells
    pub fn cachelabels(&mut self) {
        self.label_cache.clear();
        for (&(x, y, z), cell) in &self.cells {
            if let Some(ref label) = cell.label {
                self.label_cache.insert(label.clone(), (x, y, z));
            }
        }
    }

    /// Set cell contents from a token vector
    pub fn putcont(&mut self, x: usize, y: usize, z: usize, contents: Vec<Token>) {
        let cell = self.get_or_create_cell(x, y, z);
        cell.contents = Some(contents);
        cell.updated = false;
        self.changed = true;

        // Grow dimensions if needed
        if x >= self.dim_x { self.dim_x = x + 1; }
        if y >= self.dim_y { self.dim_y = y + 1; }
        if z >= self.dim_z { self.dim_z = z + 1; }
    }

    /// Get the value of a cell, evaluating its formula on demand if needed.
    pub fn getvalue(&mut self, x: usize, y: usize, z: usize) -> Token {
        // Evaluate on demand if not yet updated (lazy evaluation like C original)
        let needs_eval = self.cells.get(&(x, y, z))
            .map(|c| !c.updated && c.contents.is_some())
            .unwrap_or(false);
        if needs_eval {
            self.eval_cell(x, y, z);
        }
        if let Some(cell) = self.cells.get(&(x, y, z)) {
            cell.value.clone()
        } else {
            Token::Empty
        }
    }

    /// Recalculate all cells in the sheet
    pub fn update(&mut self) {
        // Mark all cells as not updated
        for cell in self.cells.values_mut() {
            cell.updated = false;
        }

        // Collect all coords that have contents
        let coords: Vec<(usize, usize, usize)> = self.cells.iter()
            .filter(|(_, cell)| cell.contents.is_some())
            .map(|(k, _)| *k)
            .collect();

        // Evaluate each cell
        for (x, y, z) in coords {
            self.eval_cell(x, y, z);
        }
    }

    /// Evaluate a single cell's formula and cache the result
    fn eval_cell(&mut self, x: usize, y: usize, z: usize) {
        // Check if already updated or has no contents
        let needs_eval = self.cells.get(&(x, y, z))
            .map(|c| !c.updated && c.contents.is_some())
            .unwrap_or(false);

        if !needs_eval {
            return;
        }

        // Take contents out to avoid borrow conflict
        let contents = self.cells.get_mut(&(x, y, z))
            .and_then(|c| c.contents.take());

        if let Some(tokens) = contents {
            let value = {
                let mut ctx = crate::parser::EvalContext {
                    sheet: self,
                    x, y, z,
                    max_eval: 256,
                };
                crate::parser::eval_tokens(&tokens, &mut ctx)
            };

            // Put contents back and store value
            if let Some(cell) = self.cells.get_mut(&(x, y, z)) {
                cell.contents = Some(tokens);
                cell.value = value;
                cell.updated = true;
            }
        }
    }

    /// Iterator over all cells
    pub fn cells(&self) -> impl Iterator<Item = (&(usize, usize, usize), &Cell)> {
        self.cells.iter()
    }

    /// Get all cell coordinates
    pub fn cell_coords(&self) -> Vec<(usize, usize, usize)> {
        self.cells.keys().cloned().collect()
    }

    /// Get the marked block range, normalized so (x1,y1,z1) <= (x2,y2,z2).
    /// Returns None if marks are not fully set.
    pub fn get_mark_range(&self) -> Option<(usize, usize, usize, usize, usize, usize)> {
        match (self.mark1_x, self.mark1_y, self.mark1_z,
               self.mark2_x, self.mark2_y, self.mark2_z) {
            (Some(x1), Some(y1), Some(z1), Some(x2), Some(y2), Some(z2)) => {
                Some((x1.min(x2), y1.min(y2), z1.min(z2),
                      x1.max(x2), y1.max(y2), z1.max(z2)))
            }
            _ => None,
        }
    }

    /// Clear all cells in a range
    pub fn clear_block(&mut self, x1: usize, y1: usize, z1: usize,
                       x2: usize, y2: usize, z2: usize) -> usize {
        let mut count = 0;
        let keys: Vec<_> = self.cells.keys()
            .filter(|&&(x, y, z)| x >= x1 && x <= x2 && y >= y1 && y <= y2 && z >= z1 && z <= z2)
            .cloned()
            .collect();
        for key in keys {
            self.cells.remove(&key);
            count += 1;
        }
        self.changed = true;
        count
    }

    /// Insert rows at position y, shifting existing rows down
    pub fn insert_row(&mut self, at_y: usize, z: usize) {
        // Collect cells that need shifting (y >= at_y on this sheet)
        let mut to_move: Vec<((usize, usize, usize), Cell)> = Vec::new();
        let keys: Vec<_> = self.cells.keys()
            .filter(|&&(_, y, cz)| cz == z && y >= at_y)
            .cloned()
            .collect();
        // Remove in reverse y order to avoid conflicts
        let mut keys_sorted = keys;
        keys_sorted.sort_by(|a, b| b.1.cmp(&a.1));
        for key in keys_sorted {
            if let Some(cell) = self.cells.remove(&key) {
                to_move.push(((key.0, key.1 + 1, key.2), cell));
            }
        }
        for (key, cell) in to_move {
            self.cells.insert(key, cell);
        }
        self.dim_y += 1;
        self.changed = true;
    }

    /// Delete a row, shifting rows above it down
    pub fn delete_row(&mut self, at_y: usize, z: usize) {
        // Remove cells at at_y
        let keys_at: Vec<_> = self.cells.keys()
            .filter(|&&(_, y, cz)| cz == z && y == at_y)
            .cloned()
            .collect();
        for key in keys_at {
            self.cells.remove(&key);
        }
        // Shift cells above at_y down
        let mut to_move: Vec<((usize, usize, usize), Cell)> = Vec::new();
        let keys: Vec<_> = self.cells.keys()
            .filter(|&&(_, y, cz)| cz == z && y > at_y)
            .cloned()
            .collect();
        let mut keys_sorted = keys;
        keys_sorted.sort_by(|a, b| a.1.cmp(&b.1));
        for key in keys_sorted {
            if let Some(cell) = self.cells.remove(&key) {
                to_move.push(((key.0, key.1 - 1, key.2), cell));
            }
        }
        for (key, cell) in to_move {
            self.cells.insert(key, cell);
        }
        if self.dim_y > 1 { self.dim_y -= 1; }
        self.changed = true;
    }

    /// Insert a column at position x, shifting existing columns right
    pub fn insert_col(&mut self, at_x: usize, z: usize) {
        let mut to_move: Vec<((usize, usize, usize), Cell)> = Vec::new();
        let keys: Vec<_> = self.cells.keys()
            .filter(|&&(x, _, cz)| cz == z && x >= at_x)
            .cloned()
            .collect();
        let mut keys_sorted = keys;
        keys_sorted.sort_by(|a, b| b.0.cmp(&a.0));
        for key in keys_sorted {
            if let Some(cell) = self.cells.remove(&key) {
                to_move.push(((key.0 + 1, key.1, key.2), cell));
            }
        }
        for (key, cell) in to_move {
            self.cells.insert(key, cell);
        }
        // Shift column widths
        let mut width_moves: Vec<((usize, usize), usize)> = Vec::new();
        let width_keys: Vec<_> = self.column_widths.keys()
            .filter(|&&(x, cz)| cz == z && x >= at_x)
            .cloned()
            .collect();
        let mut wk_sorted = width_keys;
        wk_sorted.sort_by(|a, b| b.0.cmp(&a.0));
        for key in wk_sorted {
            if let Some(w) = self.column_widths.remove(&key) {
                width_moves.push(((key.0 + 1, key.1), w));
            }
        }
        for (key, w) in width_moves {
            self.column_widths.insert(key, w);
        }
        self.dim_x += 1;
        self.changed = true;
    }

    /// Delete a column, shifting columns to the right left
    pub fn delete_col(&mut self, at_x: usize, z: usize) {
        // Remove cells at at_x
        let keys_at: Vec<_> = self.cells.keys()
            .filter(|&&(x, _, cz)| cz == z && x == at_x)
            .cloned()
            .collect();
        for key in keys_at {
            self.cells.remove(&key);
        }
        // Shift cells right of at_x left
        let mut to_move: Vec<((usize, usize, usize), Cell)> = Vec::new();
        let keys: Vec<_> = self.cells.keys()
            .filter(|&&(x, _, cz)| cz == z && x > at_x)
            .cloned()
            .collect();
        let mut keys_sorted = keys;
        keys_sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for key in keys_sorted {
            if let Some(cell) = self.cells.remove(&key) {
                to_move.push(((key.0 - 1, key.1, key.2), cell));
            }
        }
        for (key, cell) in to_move {
            self.cells.insert(key, cell);
        }
        // Shift column widths
        self.column_widths.remove(&(at_x, z));
        let mut width_moves: Vec<((usize, usize), usize)> = Vec::new();
        let width_keys: Vec<_> = self.column_widths.keys()
            .filter(|&&(x, cz)| cz == z && x > at_x)
            .cloned()
            .collect();
        let mut wk_sorted = width_keys;
        wk_sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for key in wk_sorted {
            if let Some(w) = self.column_widths.remove(&key) {
                width_moves.push(((key.0 - 1, key.1), w));
            }
        }
        for (key, w) in width_moves {
            self.column_widths.insert(key, w);
        }
        if self.dim_x > 1 { self.dim_x -= 1; }
        self.changed = true;
    }

    /// Copy a block of cells to a new location
    pub fn copy_block(&mut self, x1: usize, y1: usize, z1: usize,
                      x2: usize, y2: usize, z2: usize,
                      to_x: usize, to_y: usize, to_z: usize) -> usize {
        let mut copies: Vec<((usize, usize, usize), Cell)> = Vec::new();
        for (&(x, y, z), cell) in &self.cells {
            if x >= x1 && x <= x2 && y >= y1 && y <= y2 && z >= z1 && z <= z2 {
                let nx = to_x + (x - x1);
                let ny = to_y + (y - y1);
                let nz = to_z + (z - z1);
                copies.push(((nx, ny, nz), cell.clone()));
            }
        }
        let count = copies.len();
        for (key, cell) in copies {
            self.cells.insert(key, cell);
            if key.0 >= self.dim_x { self.dim_x = key.0 + 1; }
            if key.1 >= self.dim_y { self.dim_y = key.1 + 1; }
            if key.2 >= self.dim_z { self.dim_z = key.2 + 1; }
        }
        self.changed = true;
        count
    }

    /// Clear the mark
    pub fn clear_mark(&mut self) {
        self.mark1_x = None;
        self.mark1_y = None;
        self.mark1_z = None;
        self.mark2_x = None;
        self.mark2_y = None;
        self.mark2_z = None;
        self.marking = false;
    }

    /// Sort rows in a block by a key column
    pub fn sort_block(&mut self, x1: usize, y1: usize, z1: usize,
                      x2: usize, y2: usize, _z2: usize,
                      sort_col: usize, ascending: bool) {
        // Collect rows as vectors of (x, cell) pairs
        let mut rows: Vec<(usize, Vec<((usize, usize), Cell)>)> = Vec::new();
        for y in y1..=y2 {
            let mut row_cells = Vec::new();
            for x in x1..=x2 {
                if let Some(cell) = self.cells.remove(&(x, y, z1)) {
                    row_cells.push(((x, y), cell));
                }
            }
            rows.push((y, row_cells));
        }

        // Sort by the value at sort_col
        rows.sort_by(|a, b| {
            let val_a = a.1.iter()
                .find(|((x, _), _)| *x == sort_col)
                .map(|(_, c)| &c.value);
            let val_b = b.1.iter()
                .find(|((x, _), _)| *x == sort_col)
                .map(|(_, c)| &c.value);
            let ord = cmp_token_values(val_a, val_b);
            if ascending { ord } else { ord.reverse() }
        });

        // Put rows back at sequential y positions
        for (new_y_offset, (_old_y, row_cells)) in rows.into_iter().enumerate() {
            let new_y = y1 + new_y_offset;
            for ((x, _old_y), cell) in row_cells {
                self.cells.insert((x, new_y, z1), cell);
            }
        }

        self.changed = true;
        self.cachelabels();
        self.update();
    }

    /// Move a block from source to destination (copy + clear source)
    pub fn move_block(&mut self, x1: usize, y1: usize, z1: usize,
                      x2: usize, y2: usize, z2: usize,
                      to_x: usize, to_y: usize, to_z: usize) -> usize {
        // Collect source cells with relative positions
        let mut temp: Vec<((usize, usize, usize), Cell)> = Vec::new();
        for z in z1..=z2 {
            for y in y1..=y2 {
                for x in x1..=x2 {
                    if let Some(cell) = self.cells.remove(&(x, y, z)) {
                        temp.push(((x - x1, y - y1, z - z1), cell));
                    }
                }
            }
        }
        // Place at destination
        let count = temp.len();
        for ((dx, dy, dz), cell) in temp {
            let nx = to_x + dx;
            let ny = to_y + dy;
            let nz = to_z + dz;
            self.cells.insert((nx, ny, nz), cell);
            if nx >= self.dim_x { self.dim_x = nx + 1; }
            if ny >= self.dim_y { self.dim_y = ny + 1; }
            if nz >= self.dim_z { self.dim_z = nz + 1; }
        }
        self.changed = true;
        self.cachelabels();
        self.update();
        count
    }

    /// Save a snapshot of all cell data for undo (call before mutating)
    pub fn save_undo(&mut self) {
        // New action invalidates redo history
        self.redo_stack.clear();
        // Keep max 50 undo levels
        if self.undo_stack.len() >= 50 {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(UndoSnapshot {
            cells: self.cells.clone(),
            column_widths: self.column_widths.clone(),
            dim_x: self.dim_x,
            dim_y: self.dim_y,
            dim_z: self.dim_z,
        });
    }

    /// Undo the last operation, restoring the previous snapshot
    pub fn undo(&mut self) -> bool {
        if let Some(snapshot) = self.undo_stack.pop() {
            // Save current state to redo stack
            self.redo_stack.push(UndoSnapshot {
                cells: self.cells.clone(),
                column_widths: self.column_widths.clone(),
                dim_x: self.dim_x,
                dim_y: self.dim_y,
                dim_z: self.dim_z,
            });
            self.cells = snapshot.cells;
            self.column_widths = snapshot.column_widths;
            self.dim_x = snapshot.dim_x;
            self.dim_y = snapshot.dim_y;
            self.dim_z = snapshot.dim_z;
            self.cachelabels();
            self.update();
            self.changed = true;
            true
        } else {
            false
        }
    }

    /// Redo the last undone operation
    pub fn redo(&mut self) -> bool {
        if let Some(snapshot) = self.redo_stack.pop() {
            // Save current state to undo stack (without clearing redo)
            self.undo_stack.push(UndoSnapshot {
                cells: self.cells.clone(),
                column_widths: self.column_widths.clone(),
                dim_x: self.dim_x,
                dim_y: self.dim_y,
                dim_z: self.dim_z,
            });
            self.cells = snapshot.cells;
            self.column_widths = snapshot.column_widths;
            self.dim_x = snapshot.dim_x;
            self.dim_y = snapshot.dim_y;
            self.dim_z = snapshot.dim_z;
            self.cachelabels();
            self.update();
            self.changed = true;
            true
        } else {
            false
        }
    }

    /// Yank (copy) the marked block into the clipboard
    pub fn yank_block(&mut self) -> usize {
        self.clipboard.clear();
        if let Some((x1, y1, z1, x2, y2, z2)) = self.get_mark_range() {
            for z in z1..=z2 {
                for y in y1..=y2 {
                    for x in x1..=x2 {
                        if let Some(cell) = self.get_cell(x, y, z) {
                            // Store with relative position from mark origin
                            self.clipboard.push(((x - x1, y - y1, z - z1), cell.clone()));
                        }
                    }
                }
            }
            self.clipboard.len()
        } else {
            0
        }
    }

    /// Paste the clipboard at the current cursor position
    pub fn paste(&mut self) -> usize {
        if self.clipboard.is_empty() {
            return 0;
        }
        self.save_undo();
        let base_x = self.cur_x;
        let base_y = self.cur_y;
        let base_z = self.cur_z;
        let count = self.clipboard.len();
        let items: Vec<_> = self.clipboard.clone();
        for ((dx, dy, dz), cell) in items {
            let x = base_x + dx;
            let y = base_y + dy;
            let z = base_z + dz;
            let target = self.get_or_create_cell(x, y, z);
            *target = cell;
            if x >= self.dim_x { self.dim_x = x + 1; }
            if y >= self.dim_y { self.dim_y = y + 1; }
            if z >= self.dim_z { self.dim_z = z + 1; }
        }
        self.changed = true;
        self.cachelabels();
        self.update();
        count
    }

    /// Mirror a block of cells along the given axis
    pub fn mirror_block(&mut self, x1: usize, y1: usize, z1: usize,
                        x2: usize, y2: usize, z2: usize, direction: Direction) {
        match direction {
            Direction::X => {
                // Swap columns left↔right within block
                let width = x2 - x1;
                for y in y1..=y2 {
                    for z in z1..=z2 {
                        for i in 0..=(width / 2) {
                            let left = x1 + i;
                            let right = x2 - i;
                            if left >= right { break; }
                            let cell_l = self.cells.remove(&(left, y, z));
                            let cell_r = self.cells.remove(&(right, y, z));
                            if let Some(c) = cell_l { self.cells.insert((right, y, z), c); }
                            if let Some(c) = cell_r { self.cells.insert((left, y, z), c); }
                        }
                    }
                }
            }
            Direction::Y => {
                // Swap rows top↔bottom within block
                let height = y2 - y1;
                for x in x1..=x2 {
                    for z in z1..=z2 {
                        for i in 0..=(height / 2) {
                            let top = y1 + i;
                            let bot = y2 - i;
                            if top >= bot { break; }
                            let cell_t = self.cells.remove(&(x, top, z));
                            let cell_b = self.cells.remove(&(x, bot, z));
                            if let Some(c) = cell_t { self.cells.insert((x, bot, z), c); }
                            if let Some(c) = cell_b { self.cells.insert((x, top, z), c); }
                        }
                    }
                }
            }
            Direction::Z => {
                // Swap layers front↔back within block
                let depth = z2 - z1;
                for x in x1..=x2 {
                    for y in y1..=y2 {
                        for i in 0..=(depth / 2) {
                            let front = z1 + i;
                            let back = z2 - i;
                            if front >= back { break; }
                            let cell_f = self.cells.remove(&(x, y, front));
                            let cell_b = self.cells.remove(&(x, y, back));
                            if let Some(c) = cell_f { self.cells.insert((x, y, back), c); }
                            if let Some(c) = cell_b { self.cells.insert((x, y, front), c); }
                        }
                    }
                }
            }
        }
        self.changed = true;
        self.cachelabels();
        self.update();
    }

    /// Fill (tile) a block repeatedly in a grid pattern
    pub fn fill_block(&mut self, x1: usize, y1: usize, z1: usize,
                      x2: usize, y2: usize, z2: usize,
                      cols: usize, rows: usize, layers: usize) -> usize {
        let bw = x2 - x1 + 1;
        let bh = y2 - y1 + 1;
        let bd = z2 - z1 + 1;
        let mut total = 0;
        for lz in 0..layers {
            for ry in 0..rows {
                for cx in 0..cols {
                    if cx == 0 && ry == 0 && lz == 0 { continue; } // skip original
                    let to_x = x1 + cx * bw;
                    let to_y = y1 + ry * bh;
                    let to_z = z1 + lz * bd;
                    total += self.copy_block(x1, y1, z1, x2, y2, z2, to_x, to_y, to_z);
                }
            }
        }
        self.update();
        total
    }

    /// Sort columns in a block by a key row
    pub fn sort_block_y(&mut self, x1: usize, y1: usize, z1: usize,
                        x2: usize, y2: usize, _z2: usize,
                        sort_row: usize, ascending: bool) {
        // Collect columns as vectors of (y, cell) pairs
        let mut cols: Vec<(usize, Vec<((usize, usize), Cell)>)> = Vec::new();
        for x in x1..=x2 {
            let mut col_cells = Vec::new();
            for y in y1..=y2 {
                if let Some(cell) = self.cells.remove(&(x, y, z1)) {
                    col_cells.push(((x, y), cell));
                }
            }
            cols.push((x, col_cells));
        }

        cols.sort_by(|a, b| {
            let val_a = a.1.iter()
                .find(|((_, y), _)| *y == sort_row)
                .map(|(_, c)| &c.value);
            let val_b = b.1.iter()
                .find(|((_, y), _)| *y == sort_row)
                .map(|(_, c)| &c.value);
            let ord = cmp_token_values(val_a, val_b);
            if ascending { ord } else { ord.reverse() }
        });

        for (new_x_offset, (_old_x, col_cells)) in cols.into_iter().enumerate() {
            let new_x = x1 + new_x_offset;
            for ((_old_x, y), cell) in col_cells {
                self.cells.insert((new_x, y, z1), cell);
            }
        }

        self.changed = true;
        self.cachelabels();
        self.update();
    }

    /// Sort layers in a block by a cell value at a given layer
    pub fn sort_block_z(&mut self, x1: usize, y1: usize, z1: usize,
                        x2: usize, y2: usize, z2: usize,
                        sort_x: usize, sort_y: usize, ascending: bool) {
        // Collect layers
        let mut layers: Vec<(usize, Vec<((usize, usize, usize), Cell)>)> = Vec::new();
        for z in z1..=z2 {
            let mut layer_cells = Vec::new();
            for y in y1..=y2 {
                for x in x1..=x2 {
                    if let Some(cell) = self.cells.remove(&(x, y, z)) {
                        layer_cells.push(((x, y, z), cell));
                    }
                }
            }
            layers.push((z, layer_cells));
        }

        layers.sort_by(|a, b| {
            let val_a = a.1.iter()
                .find(|((x, y, _), _)| *x == sort_x && *y == sort_y)
                .map(|(_, c)| &c.value);
            let val_b = b.1.iter()
                .find(|((x, y, _), _)| *x == sort_x && *y == sort_y)
                .map(|(_, c)| &c.value);
            let ord = cmp_token_values(val_a, val_b);
            if ascending { ord } else { ord.reverse() }
        });

        for (new_z_offset, (_old_z, layer_cells)) in layers.into_iter().enumerate() {
            let new_z = z1 + new_z_offset;
            for ((x, y, _old_z), cell) in layer_cells {
                self.cells.insert((x, y, new_z), cell);
            }
        }

        self.changed = true;
        self.cachelabels();
        self.update();
    }

    /// Toggle clock mode on a cell
    pub fn toggle_clock(&mut self, x: usize, y: usize, z: usize) -> bool {
        let cell = self.get_or_create_cell(x, y, z);
        cell.clock_t2 = !cell.clock_t2;
        if cell.clock_t2 {
            // Copy current contents to clocked_contents
            cell.clocked_contents = cell.contents.clone();
        } else {
            cell.clocked_contents = None;
            cell.clock_t0 = false;
            cell.clock_t1 = false;
        }
        cell.clock_t2
    }

    /// Search cells on a sheet layer, return matching coordinates sorted by (z, y, x)
    pub fn search_cells(&self, z: usize, pattern: &Regex, search_values: bool) -> Vec<(usize, usize, usize)> {
        let mut results: Vec<(usize, usize, usize)> = self.cells.iter()
            .filter(|(&(_, _, cz), _)| cz == z)
            .filter(|(&(x, y, z), cell)| {
                let text = if search_values {
                    cell.value.to_string()
                } else {
                    cell.contents.as_ref()
                        .map(|c| crate::scanner::print_tokens(c, true, cell.scientific, cell.precision))
                        .unwrap_or_default()
                };
                let _ = (x, y, z); // suppress unused
                pattern.is_match(&text)
            })
            .map(|(&k, _)| k)
            .collect();
        results.sort_by(|a, b| a.2.cmp(&b.2).then(a.1.cmp(&b.1)).then(a.0.cmp(&b.0)));
        results
    }

    /// Search all layers, return matching coordinates sorted by (z, y, x)
    pub fn search_all_cells(&self, pattern: &Regex, search_values: bool) -> Vec<(usize, usize, usize)> {
        let mut results: Vec<(usize, usize, usize)> = self.cells.iter()
            .filter(|(_, cell)| {
                let text = if search_values {
                    cell.value.to_string()
                } else {
                    cell.contents.as_ref()
                        .map(|c| crate::scanner::print_tokens(c, true, cell.scientific, cell.precision))
                        .unwrap_or_default()
                };
                pattern.is_match(&text)
            })
            .map(|(&k, _)| k)
            .collect();
        results.sort_by(|a, b| a.2.cmp(&b.2).then(a.1.cmp(&b.1)).then(a.0.cmp(&b.0)));
        results
    }

    /// Replace cell content matching pattern. Returns true if replacement was made.
    pub fn replace_cell(&mut self, x: usize, y: usize, z: usize, pattern: &Regex, replacement: &str, in_values: bool) -> bool {
        let cell = match self.cells.get(&(x, y, z)) {
            Some(c) => c,
            None => return false,
        };

        if in_values {
            // Replace in the display value: re-scan the replaced string as new content
            let text = cell.value.to_string();
            let is_string_value = matches!(cell.value, Token::String(_));
            if pattern.is_match(&text) {
                let new_text = pattern.replace_all(&text, replacement).to_string();
                // If the original was a string value, wrap in quotes
                let scan_input = if is_string_value {
                    format!("\"{}\"", new_text.replace('"', "\\\""))
                } else {
                    new_text
                };
                let new_contents = match crate::scanner::scan(&scan_input) {
                    Ok(tokens) => tokens,
                    Err(_) => return false,
                };
                self.putcont(x, y, z, new_contents);
                return true;
            }
        } else {
            // Replace in formula text
            let formula = cell.contents.as_ref()
                .map(|c| crate::scanner::print_tokens(c, true, cell.scientific, cell.precision))
                .unwrap_or_default();
            if pattern.is_match(&formula) {
                let new_formula = pattern.replace_all(&formula, replacement).to_string();
                match crate::scanner::scan(&new_formula) {
                    Ok(tokens) => {
                        self.putcont(x, y, z, tokens);
                        return true;
                    }
                    Err(_) => return false,
                }
            }
        }
        false
    }

    /// Execute one clock tick: trigger → evaluate → commit
    pub fn clock_tick(&mut self) -> usize {
        // Phase 1: Mark clocked cells for re-eval
        let clocked_coords: Vec<(usize, usize, usize)> = self.cells.iter()
            .filter(|(_, cell)| cell.clock_t2)
            .map(|(k, _)| *k)
            .collect();

        if clocked_coords.is_empty() {
            return 0;
        }

        for &(x, y, z) in &clocked_coords {
            if let Some(cell) = self.cells.get_mut(&(x, y, z)) {
                cell.clock_t0 = true;
                cell.clock_t1 = true;
                cell.updated = false;
            }
        }

        // Phase 2: Evaluate all clocked cells, store in clocked_value
        for &(x, y, z) in &clocked_coords {
            let contents = self.cells.get_mut(&(x, y, z))
                .and_then(|c| c.contents.take());

            if let Some(tokens) = contents {
                let value = {
                    let mut ctx = crate::parser::EvalContext {
                        sheet: self,
                        x, y, z,
                        max_eval: 256,
                    };
                    crate::parser::eval_tokens(&tokens, &mut ctx)
                };

                if let Some(cell) = self.cells.get_mut(&(x, y, z)) {
                    cell.contents = Some(tokens);
                    cell.clocked_value = value;
                }
            }
        }

        // Phase 3: Commit - copy clocked_value → value, clear flags
        for &(x, y, z) in &clocked_coords {
            if let Some(cell) = self.cells.get_mut(&(x, y, z)) {
                cell.value = cell.clocked_value.clone();
                cell.clock_t0 = false;
                cell.clock_t1 = false;
                cell.updated = true;
            }
        }

        self.changed = true;
        clocked_coords.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_delete_row() {
        let mut sheet = Sheet::new();
        sheet.putcont(0, 0, 0, vec![Token::Integer(1)]);
        sheet.putcont(0, 1, 0, vec![Token::Integer(2)]);
        sheet.putcont(0, 2, 0, vec![Token::Integer(3)]);
        sheet.update();

        // Insert row at y=1, should shift rows 1,2 down
        sheet.insert_row(1, 0);
        assert!(sheet.get_cell(0, 1, 0).is_none()); // new empty row
        assert_eq!(sheet.get_cell(0, 0, 0).unwrap().value, Token::Integer(1));
        assert_eq!(sheet.get_cell(0, 2, 0).unwrap().value, Token::Integer(2));
        assert_eq!(sheet.get_cell(0, 3, 0).unwrap().value, Token::Integer(3));

        // Delete row at y=1, should shift back
        sheet.delete_row(1, 0);
        assert_eq!(sheet.get_cell(0, 0, 0).unwrap().value, Token::Integer(1));
        assert_eq!(sheet.get_cell(0, 1, 0).unwrap().value, Token::Integer(2));
        assert_eq!(sheet.get_cell(0, 2, 0).unwrap().value, Token::Integer(3));
    }

    #[test]
    fn test_insert_delete_col() {
        let mut sheet = Sheet::new();
        sheet.putcont(0, 0, 0, vec![Token::Integer(1)]);
        sheet.putcont(1, 0, 0, vec![Token::Integer(2)]);
        sheet.putcont(2, 0, 0, vec![Token::Integer(3)]);
        sheet.update();

        sheet.insert_col(1, 0);
        assert_eq!(sheet.get_cell(0, 0, 0).unwrap().value, Token::Integer(1));
        assert!(sheet.get_cell(1, 0, 0).is_none());
        assert_eq!(sheet.get_cell(2, 0, 0).unwrap().value, Token::Integer(2));
        assert_eq!(sheet.get_cell(3, 0, 0).unwrap().value, Token::Integer(3));

        sheet.delete_col(1, 0);
        assert_eq!(sheet.get_cell(0, 0, 0).unwrap().value, Token::Integer(1));
        assert_eq!(sheet.get_cell(1, 0, 0).unwrap().value, Token::Integer(2));
        assert_eq!(sheet.get_cell(2, 0, 0).unwrap().value, Token::Integer(3));
    }

    #[test]
    fn test_clear_block() {
        let mut sheet = Sheet::new();
        sheet.putcont(0, 0, 0, vec![Token::Integer(1)]);
        sheet.putcont(1, 0, 0, vec![Token::Integer(2)]);
        sheet.putcont(0, 1, 0, vec![Token::Integer(3)]);
        sheet.putcont(1, 1, 0, vec![Token::Integer(4)]);
        sheet.update();

        let count = sheet.clear_block(0, 0, 0, 1, 0, 0);
        assert_eq!(count, 2);
        assert!(sheet.get_cell(0, 0, 0).is_none());
        assert!(sheet.get_cell(1, 0, 0).is_none());
        assert_eq!(sheet.get_cell(0, 1, 0).unwrap().value, Token::Integer(3));
    }

    #[test]
    fn test_copy_block() {
        let mut sheet = Sheet::new();
        sheet.putcont(0, 0, 0, vec![Token::Integer(1)]);
        sheet.putcont(1, 0, 0, vec![Token::Integer(2)]);
        sheet.update();

        let count = sheet.copy_block(0, 0, 0, 1, 0, 0, 0, 1, 0);
        assert_eq!(count, 2);
        assert_eq!(sheet.get_cell(0, 1, 0).unwrap().value, Token::Integer(1));
        assert_eq!(sheet.get_cell(1, 1, 0).unwrap().value, Token::Integer(2));
        // Originals still there
        assert_eq!(sheet.get_cell(0, 0, 0).unwrap().value, Token::Integer(1));
    }

    #[test]
    fn test_mirror_block_x() {
        let mut sheet = Sheet::new();
        sheet.putcont(0, 0, 0, vec![Token::Integer(1)]);
        sheet.putcont(1, 0, 0, vec![Token::Integer(2)]);
        sheet.putcont(2, 0, 0, vec![Token::Integer(3)]);
        sheet.update();

        sheet.mirror_block(0, 0, 0, 2, 0, 0, Direction::X);
        assert_eq!(sheet.get_cell(0, 0, 0).unwrap().value, Token::Integer(3));
        assert_eq!(sheet.get_cell(1, 0, 0).unwrap().value, Token::Integer(2));
        assert_eq!(sheet.get_cell(2, 0, 0).unwrap().value, Token::Integer(1));
    }

    #[test]
    fn test_mirror_block_y() {
        let mut sheet = Sheet::new();
        sheet.putcont(0, 0, 0, vec![Token::Integer(1)]);
        sheet.putcont(0, 1, 0, vec![Token::Integer(2)]);
        sheet.putcont(0, 2, 0, vec![Token::Integer(3)]);
        sheet.update();

        sheet.mirror_block(0, 0, 0, 0, 2, 0, Direction::Y);
        assert_eq!(sheet.get_cell(0, 0, 0).unwrap().value, Token::Integer(3));
        assert_eq!(sheet.get_cell(0, 1, 0).unwrap().value, Token::Integer(2));
        assert_eq!(sheet.get_cell(0, 2, 0).unwrap().value, Token::Integer(1));
    }

    #[test]
    fn test_fill_block() {
        let mut sheet = Sheet::new();
        sheet.putcont(0, 0, 0, vec![Token::Integer(1)]);
        sheet.putcont(1, 0, 0, vec![Token::Integer(2)]);
        sheet.putcont(0, 1, 0, vec![Token::Integer(3)]);
        sheet.putcont(1, 1, 0, vec![Token::Integer(4)]);
        sheet.update();

        let count = sheet.fill_block(0, 0, 0, 1, 1, 0, 2, 2, 1);
        // Should have copied to 3 additional positions (2x2 grid minus original)
        assert!(count > 0);
        // Check one of the copies
        assert_eq!(sheet.get_cell(2, 0, 0).unwrap().value, Token::Integer(1));
        assert_eq!(sheet.get_cell(3, 0, 0).unwrap().value, Token::Integer(2));
        assert_eq!(sheet.get_cell(0, 2, 0).unwrap().value, Token::Integer(1));
    }

    #[test]
    fn test_sort_block_y() {
        let mut sheet = Sheet::new();
        // 3 columns, 2 rows; sort columns by row 0
        sheet.putcont(0, 0, 0, vec![Token::Integer(3)]);
        sheet.putcont(1, 0, 0, vec![Token::Integer(1)]);
        sheet.putcont(2, 0, 0, vec![Token::Integer(2)]);
        sheet.putcont(0, 1, 0, vec![Token::String("c".to_string())]);
        sheet.putcont(1, 1, 0, vec![Token::String("a".to_string())]);
        sheet.putcont(2, 1, 0, vec![Token::String("b".to_string())]);
        sheet.update();

        sheet.sort_block_y(0, 0, 0, 2, 1, 0, 0, true);
        assert_eq!(sheet.get_cell(0, 0, 0).unwrap().value, Token::Integer(1));
        assert_eq!(sheet.get_cell(1, 0, 0).unwrap().value, Token::Integer(2));
        assert_eq!(sheet.get_cell(2, 0, 0).unwrap().value, Token::Integer(3));
    }

    #[test]
    fn test_clock_tick() {
        let mut sheet = Sheet::new();
        sheet.putcont(0, 0, 0, vec![Token::Integer(42)]);
        sheet.update();

        // Enable clock on cell
        let enabled = sheet.toggle_clock(0, 0, 0);
        assert!(enabled);
        assert!(sheet.get_cell(0, 0, 0).unwrap().clock_t2);

        // Tick should evaluate and commit
        let count = sheet.clock_tick();
        assert_eq!(count, 1);
        assert_eq!(sheet.get_cell(0, 0, 0).unwrap().value, Token::Integer(42));

        // Disable clock
        let enabled = sheet.toggle_clock(0, 0, 0);
        assert!(!enabled);
    }
}
