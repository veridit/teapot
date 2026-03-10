//! SC (Spreadsheet Calculator) file format handler

use anyhow::{bail, Result};
use std::fs;
use std::io::BufRead;
use crate::scanner;
use crate::sheet::{Adjust, Sheet};

/// Parse an SC column reference like "A", "B", ..., "Z", "AA", "AB", etc.
fn parse_col(s: &str) -> Option<(usize, &str)> {
    let bytes = s.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_uppercase() {
        return None;
    }
    let mut col = (bytes[0] - b'A') as usize;
    let mut i = 1;
    if i < bytes.len() && bytes[i].is_ascii_uppercase() {
        col = col * 26 + (bytes[i] - b'A') as usize;
        i += 1;
    }
    Some((col, &s[i..]))
}

/// Parse an SC row number
fn parse_row(s: &str) -> Option<(usize, &str)> {
    let bytes = s.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_digit() {
        return None;
    }
    let mut row = 0usize;
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        row = row * 10 + (bytes[i] - b'0') as usize;
        i += 1;
    }
    Some((row, &s[i..]))
}

/// Parse a cell reference like "A0", "BC123"
fn parse_cell_ref(s: &str) -> Option<(usize, usize, &str)> {
    let (col, rest) = parse_col(s)?;
    let (row, rest) = parse_row(rest)?;
    Some((col, row, rest))
}

/// Translate an SC expression to a Teapot expression.
/// SC uses @func() syntax and A0-style cell references.
fn translate_expression(expr: &str) -> String {
    let mut result = String::new();
    let mut chars = expr.chars().peekable();

    while let Some(&c) = chars.peek() {
        if c == '@' {
            chars.next();
            // Collect function name
            let mut fname = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_ascii_alphabetic() {
                    fname.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            // Map SC functions to Teapot
            match fname.as_str() {
                "sum" => result.push_str("sum"),
                "rnd" => result.push_str("int"),  // approximate
                "floor" => result.push_str("int"),
                "ceil" => result.push_str("int"),
                "abs" => result.push_str("abs"),
                "sqrt" => result.push_str("abs"),  // no sqrt, approximate
                "sin" => result.push_str("sin"),
                "cos" => result.push_str("cos"),
                "tan" => result.push_str("tan"),
                "log" => result.push_str("log"),
                "exp" => result.push_str("e"),
                "min" => result.push_str("min"),
                "max" => result.push_str("max"),
                _ => result.push_str(&fname),
            }
        } else if c.is_ascii_uppercase() {
            // Try to parse cell reference
            let remaining: String = chars.clone().collect();
            let full = format!("{}{}", c, remaining);
            if let Some((col, row, rest)) = parse_cell_ref(&full) {
                result.push_str(&format!("@({},{},0)", col, row));
                // Advance chars past the cell reference
                let consumed = full.len() - rest.len();
                for _ in 0..consumed {
                    chars.next();
                }
            } else {
                result.push(c);
                chars.next();
            }
        } else if c == ':' {
            // Range separator -> comma in Teapot
            result.push(',');
            chars.next();
        } else {
            result.push(c);
            chars.next();
        }
    }

    result
}

/// Load a sheet from an SC file
pub fn load_sc(sheet: &mut Sheet, name: &str) -> Result<()> {
    let file = fs::File::open(name)?;
    let reader = std::io::BufReader::new(file);
    let mut line_num = 0;

    for line_result in reader.lines() {
        let line = line_result?;
        line_num += 1;

        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(rest) = line.strip_prefix("format ") {
            // format COL WIDTH PRECISION WHOKNOWS
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() >= 3 {
                if let Some((col, _)) = parse_col(parts[0]) {
                    if let (Ok(width), Ok(precision)) = (parts[1].parse::<usize>(), parts[2].parse::<i32>()) {
                        sheet.set_width(col, 0, width);
                        let cell = sheet.get_or_create_cell(col, 0, 0);
                        cell.adjust = Adjust::Right;
                        cell.precision = precision;
                    }
                }
            }
        } else if line.starts_with("leftstring ") || line.starts_with("rightstring ") {
            let is_left = line.starts_with("leftstring ");
            let rest = if is_left { &line[11..] } else { &line[12..] };

            if let Some((col, row, rest)) = parse_cell_ref(rest) {
                // Skip " = "
                let rest = rest.trim_start();
                let rest = rest.strip_prefix('=').unwrap_or(rest).trim_start();

                // The value is a quoted string
                let tokens = scanner::scan(rest)?;
                let cell = sheet.get_or_create_cell(col, row, 0);
                cell.adjust = if is_left { Adjust::Left } else { Adjust::Right };
                cell.contents = Some(tokens);

                if col >= sheet.dim_x { sheet.dim_x = col + 1; }
                if row >= sheet.dim_y { sheet.dim_y = row + 1; }
            }
        } else if let Some(rest) = line.strip_prefix("let ") {
            if let Some((col, row, rest)) = parse_cell_ref(rest) {
                let rest = rest.trim_start();
                let rest = rest.strip_prefix('=').unwrap_or(rest).trim_start();

                let translated = translate_expression(rest);
                match scanner::scan(&translated) {
                    Ok(tokens) => {
                        let cell = sheet.get_or_create_cell(col, row, 0);
                        cell.adjust = Adjust::Right;
                        cell.contents = Some(tokens);

                        if col >= sheet.dim_x { sheet.dim_x = col + 1; }
                        if row >= sheet.dim_y { sheet.dim_y = row + 1; }
                    }
                    Err(e) => {
                        bail!("Expression syntax error in line {}: {}", line_num, e);
                    }
                }
            }
        }
        // Silently ignore other lines (define, goto, etc.)
    }

    // Apply column precision to all rows
    for x in 0..sheet.dim_x {
        let prec = sheet.get_cell(x, 0, 0)
            .map(|c| if c.precision == -1 { 2 } else { c.precision })
            .unwrap_or(2);
        for y in 1..sheet.dim_y {
            if let Some(cell) = sheet.get_cell_mut(x, y, 0) {
                if cell.precision == -1 {
                    cell.precision = prec;
                }
            }
        }
    }

    sheet.changed = false;
    sheet.cachelabels();
    sheet.update();
    Ok(())
}
