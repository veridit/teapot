//! Sheet module - defines the spreadsheet data structure

use crate::token::Token;
use std::collections::HashMap;

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
            adjust: Adjust::Left,
            precision: 6,
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

    /// Get the value of a cell, evaluating its formula if needed.
    /// For now, returns the cached value (formula evaluation requires update() to run first).
    pub fn getvalue(&mut self, x: usize, y: usize, z: usize) -> Token {
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
}
