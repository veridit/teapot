use anyhow::{bail, Result};
use std::fs;
use std::io::{BufRead, BufReader, Write};

use crate::scanner;
use crate::sheet::{Adjust, Sheet};
use crate::token::Token;

pub mod html;
pub mod latex;
pub mod context;
pub mod sc;
pub mod wk1;

/// Load a sheet from an XDR file
pub fn load_xdr(_sheet: &mut Sheet, _filename: &str) -> Result<()> {
    bail!("XDR format not yet implemented")
}

/// Save a sheet to an XDR file
pub fn save_xdr(_sheet: &Sheet, _filename: &str) -> Result<usize> {
    bail!("XDR format not yet implemented")
}

/// Load a sheet from a portable ASCII file (.tpa)
pub fn load_port(sheet: &mut Sheet, filename: &str) -> Result<()> {
    let file = fs::File::open(filename)?;
    let reader = BufReader::new(file);
    let mut line_num = 0;

    for line_result in reader.lines() {
        let line = line_result?;
        line_num += 1;

        if line.is_empty() {
            continue;
        }

        match line.as_bytes()[0] {
            b'#' => { /* comment, skip */ }
            b'W' => parse_width_line(&line[1..], sheet, line_num)?,
            b'C' => parse_cell_line(&line[1..], sheet, line_num)?,
            c => bail!("Unknown tag '{}' in line {}", c as char, line_num),
        }
    }

    sheet.changed = false;
    sheet.cachelabels();
    sheet.update();
    Ok(())
}

fn parse_width_line(s: &str, sheet: &mut Sheet, line_num: usize) -> Result<()> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() < 3 {
        bail!("Parse error for width in line {}", line_num);
    }
    let x: usize = parts[0].parse().map_err(|_| anyhow::anyhow!("Parse error for x in line {}", line_num))?;
    let z: usize = parts[1].parse().map_err(|_| anyhow::anyhow!("Parse error for z in line {}", line_num))?;
    let width: usize = parts[2].parse().map_err(|_| anyhow::anyhow!("Parse error for width in line {}", line_num))?;
    sheet.set_width(x, z, width);
    Ok(())
}

fn parse_cell_line(s: &str, sheet: &mut Sheet, line_num: usize) -> Result<()> {
    let mut chars = s.char_indices().peekable();

    // Helper to skip spaces
    fn skip_spaces(chars: &mut std::iter::Peekable<std::str::CharIndices>, s: &str) {
        while let Some(&(_, c)) = chars.peek() {
            if c == ' ' { chars.next(); } else { break; }
        }
    }

    // Helper to parse an unsigned integer
    fn parse_uint(chars: &mut std::iter::Peekable<std::str::CharIndices>, s: &str) -> Option<usize> {
        let start = chars.peek().map(|&(i, _)| i)?;
        let mut end = start;
        while let Some(&(i, c)) = chars.peek() {
            if c.is_ascii_digit() { end = i + c.len_utf8(); chars.next(); } else { break; }
        }
        if end == start { return None; }
        s[start..end].parse().ok()
    }

    // Parse x y z
    let x = parse_uint(&mut chars, s).ok_or_else(|| anyhow::anyhow!("Parse error for x in line {}", line_num))?;
    skip_spaces(&mut chars, s);
    let y = parse_uint(&mut chars, s).ok_or_else(|| anyhow::anyhow!("Parse error for y in line {}", line_num))?;
    skip_spaces(&mut chars, s);
    let z = parse_uint(&mut chars, s).ok_or_else(|| anyhow::anyhow!("Parse error for z in line {}", line_num))?;

    let cell = sheet.get_or_create_cell(x, y, z);

    // Parse attributes until ':' or end
    loop {
        skip_spaces(&mut chars, s);
        match chars.peek().map(|&(_, c)| c) {
            None | Some(':') => break,
            Some('A') => {
                chars.next(); // skip 'A'
                match chars.peek().map(|&(_, c)| c) {
                    Some('l') => { cell.adjust = Adjust::Left; chars.next(); }
                    Some('r') => { cell.adjust = Adjust::Right; chars.next(); }
                    Some('c') => { cell.adjust = Adjust::Center; chars.next(); }
                    _ => bail!("Parse error for adjustment in line {}", line_num),
                }
            }
            Some('L') => {
                chars.next(); // skip 'L'
                let start = chars.peek().map(|&(i, _)| i).unwrap_or(s.len());
                let mut end = start;
                while let Some(&(i, c)) = chars.peek() {
                    if c != ' ' { end = i + c.len_utf8(); chars.next(); } else { break; }
                }
                cell.label = Some(s[start..end].to_string());
            }
            Some('P') => {
                chars.next(); // skip 'P'
                let prec = parse_uint(&mut chars, s)
                    .ok_or_else(|| anyhow::anyhow!("Parse error for precision in line {}", line_num))?;
                cell.precision = prec as i32;
            }
            Some('S') => { chars.next(); cell.shadowed = true; }
            Some('B') => { chars.next(); cell.bold = true; }
            Some('U') => { chars.next(); cell.underline = true; }
            Some('E') => { chars.next(); cell.scientific = true; }
            Some('C') => { chars.next(); cell.locked = true; }  // 'C' means locked in file format
            Some('O') => { chars.next(); cell.locked = true; }  // 'O' also means locked
            Some('T') => { chars.next(); cell.transparent = true; }
            Some('I') => { chars.next(); cell.ignored = true; }
            Some(c) => bail!("Invalid option '{}' in line {}", c, line_num),
        }
    }

    // Parse contents after ':'
    if let Some(&(i, ':')) = chars.peek() {
        chars.next(); // skip ':'
        let rest = &s[i + 1..];
        // Check for backslash continuation (clocked contents)
        let (expr_str, _clocked_str) = if rest.ends_with('\\') {
            (&rest[..rest.len() - 1], Some("")) // continuation line handled at caller level
        } else {
            (rest, None)
        };

        let tokens = scanner::scan(expr_str)?;
        // Get the cell again (we need to re-borrow after using sheet)
        let cell = sheet.get_or_create_cell(x, y, z);
        cell.contents = Some(tokens);
    }

    // Grow dimensions
    if x >= sheet.dim_x { sheet.dim_x = x + 1; }
    if y >= sheet.dim_y { sheet.dim_y = y + 1; }
    if z >= sheet.dim_z { sheet.dim_z = z + 1; }

    Ok(())
}

/// Save a sheet to a portable ASCII file (.tpa)
pub fn save_port(sheet: &Sheet, filename: &str) -> Result<usize> {
    let mut file = fs::File::create(filename)?;
    let mut count = 0;

    writeln!(file, "# This is a work sheet generated with teapot 2.3.0.")?;

    // Write column widths
    for z in 0..sheet.dim_z {
        for x in 0..sheet.dim_x {
            let w = sheet.column_width(x, z);
            if w != sheet.width {
                writeln!(file, "W{} {} {}", x, z, w)?;
            }
        }
    }

    // Write cells
    for z in 0..sheet.dim_z {
        for y in 0..sheet.dim_y {
            for x in 0..sheet.dim_x {
                if let Some(cell) = sheet.get_cell(x, y, z) {
                    // Skip completely default/empty cells
                    let has_content = cell.contents.is_some()
                        || cell.label.is_some()
                        || cell.adjust != Adjust::AutoAdjust
                        || cell.shadowed || cell.bold || cell.underline
                        || cell.scientific || cell.locked || cell.transparent
                        || cell.ignored
                        || cell.precision != -1;
                    if !has_content {
                        continue;
                    }

                    write!(file, "C{} {} {} ", x, y, z)?;

                    match cell.adjust {
                        Adjust::Left => write!(file, "Al ")?,
                        Adjust::Right => write!(file, "Ar ")?,
                        Adjust::Center => write!(file, "Ac ")?,
                        Adjust::AutoAdjust => {}
                    }
                    if let Some(ref label) = cell.label {
                        write!(file, "L{} ", label)?;
                    }
                    if cell.precision != -1 {
                        write!(file, "P{} ", cell.precision)?;
                    }
                    if cell.shadowed { write!(file, "S ")?; }
                    if cell.bold { write!(file, "B ")?; }
                    if cell.underline { write!(file, "U ")?; }
                    if cell.scientific { write!(file, "E ")?; }
                    if cell.locked { write!(file, "C ")?; }
                    if cell.transparent { write!(file, "T ")?; }
                    if cell.ignored { write!(file, "I ")?; }

                    if let Some(ref contents) = cell.contents {
                        let expr = scanner::print_tokens(contents, true, cell.scientific, cell.precision);
                        write!(file, ":{}", expr)?;
                    }

                    if let Some(ref ccontents) = cell.clocked_contents {
                        let expr = scanner::print_tokens(ccontents, true, cell.scientific, cell.precision);
                        write!(file, "\\\n{}", expr)?;
                    }

                    writeln!(file)?;
                    count += 1;
                }
            }
        }
    }

    Ok(count)
}

/// Load a sheet from a CSV file
pub fn load_csv(sheet: &mut Sheet, filename: &str) -> Result<()> {
    let file = fs::File::open(filename)?;
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(file);

    for (y, result) in reader.records().enumerate() {
        let record = result?;
        for (x, field) in record.iter().enumerate() {
            if field.is_empty() {
                continue;
            }
            let token = if let Ok(i) = field.parse::<i64>() {
                vec![Token::Integer(i)]
            } else if let Ok(f) = field.parse::<f64>() {
                vec![Token::Float(f)]
            } else {
                vec![Token::String(field.to_string())]
            };
            sheet.putcont(x, y, 0, token);
        }
    }

    sheet.changed = false;
    sheet.update();
    Ok(())
}

/// Save a sheet to a CSV file
pub fn save_csv(
    sheet: &Sheet,
    name: &str,
    separator: char,
    x1: usize, y1: usize, z1: usize,
    x2: usize, y2: usize, _z2: usize,
) -> Result<usize> {
    let mut file = fs::File::create(name)?;
    let mut count = 0;

    for y in y1..=y2 {
        let mut first = true;
        for x in x1..=x2 {
            if !first {
                write!(file, "{}", separator)?;
            }
            first = false;
            if let Some(cell) = sheet.get_cell(x, y, z1) {
                match &cell.value {
                    Token::Empty => {}
                    Token::Integer(i) => write!(file, "{}", i)?,
                    Token::Float(f) => write!(file, "{}", f)?,
                    Token::String(s) => {
                        // Quote strings containing separator or quotes
                        if s.contains(separator) || s.contains('"') {
                            write!(file, "\"{}\"", s.replace('"', "\"\""))?;
                        } else {
                            write!(file, "{}", s)?;
                        }
                    }
                    _ => {}
                }
                count += 1;
            }
        }
        writeln!(file)?;
    }

    Ok(count)
}

/// Save a sheet to a text file
pub fn save_text(
    sheet: &Sheet,
    name: &str,
    x1: usize, y1: usize, z1: usize,
    x2: usize, y2: usize, _z2: usize,
) -> Result<usize> {
    let mut file = fs::File::create(name)?;
    let mut count = 0;

    for y in y1..=y2 {
        for x in x1..=x2 {
            let width = sheet.column_width(x, z1);
            if let Some(cell) = sheet.get_cell(x, y, z1) {
                let text = match &cell.value {
                    Token::Empty => String::new(),
                    Token::Integer(i) => format!("{}", i),
                    Token::Float(f) => format!("{}", f),
                    Token::String(s) => s.clone(),
                    _ => String::new(),
                };
                write!(file, "{:width$}", text, width = width)?;
                count += 1;
            } else {
                write!(file, "{:width$}", "", width = width)?;
            }
        }
        writeln!(file)?;
    }

    Ok(count)
}

pub use html::save_html;
pub use latex::save_latex;
pub use context::save_context;
pub use sc::load_sc;
pub use wk1::load_wk1;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn test_save_load_roundtrip() {
        let mut sheet = Sheet::new();
        sheet.putcont(0, 0, 0, vec![Token::Integer(42)]);
        sheet.putcont(1, 0, 0, vec![Token::String("hello".to_string())]);
        sheet.putcont(0, 1, 0, vec![Token::Float(3.14)]);
        sheet.update();

        let tmpfile = "/tmp/teapot_test_roundtrip.tpa";
        save_port(&sheet, tmpfile).unwrap();

        let mut sheet2 = Sheet::new();
        load_port(&mut sheet2, tmpfile).unwrap();

        // Check the values were loaded
        assert_eq!(sheet2.get_cell(0, 0, 0).unwrap().value, Token::Integer(42));
        assert_eq!(sheet2.get_cell(1, 0, 0).unwrap().value, Token::String("hello".to_string()));

        // Clean up
        std::fs::remove_file(tmpfile).ok();
    }

    #[test]
    fn test_save_port_format() {
        let mut sheet = Sheet::new();
        sheet.putcont(0, 0, 0, vec![Token::Integer(42)]);
        {
            let cell = sheet.get_or_create_cell(0, 0, 0);
            cell.adjust = Adjust::Right;
            cell.label = Some("test".to_string());
        }
        sheet.update();

        let tmpfile = "/tmp/teapot_test_format.tpa";
        save_port(&sheet, tmpfile).unwrap();

        let mut content = String::new();
        fs::File::open(tmpfile).unwrap().read_to_string(&mut content).unwrap();

        assert!(content.contains("C0 0 0 Ar Ltest :42"));

        std::fs::remove_file(tmpfile).ok();
    }
}
