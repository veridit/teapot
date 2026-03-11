//! TBL (troff tbl) export format handler

use anyhow::Result;
use std::io::Write;

use crate::sheet::{Adjust, Sheet};
use crate::token::Token;

/// Escape a string for tbl output
fn escape_tbl(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    let mut first = true;

    while let Some(c) = chars.next() {
        if first {
            first = false;
            match c {
                '.' => { out.push_str("\\&."); continue; }
                '\'' => { out.push_str("\\&'"); continue; }
                _ => {}
            }
        }
        match c {
            '\\' => out.push_str("\\e"),
            '_' => out.push_str("\\&_"),
            '-' if chars.peek() != Some(&'-') && !out.ends_with('-') => {
                // Single dash → \- but -- stays as --
                out.push_str("\\-");
            }
            _ => out.push(c),
        }
    }
    out
}

fn format_value(sheet: &Sheet, x: usize, y: usize, z: usize) -> String {
    match sheet.get_cell(x, y, z) {
        Some(cell) => match &cell.value {
            Token::Empty => String::new(),
            Token::Float(f) => {
                if cell.precision >= 0 {
                    format!("{:.*}", cell.precision as usize, f)
                } else {
                    format!("{}", f)
                }
            }
            other => other.to_string(),
        },
        None => String::new(),
    }
}

fn get_format_char(sheet: &Sheet, x: usize, y: usize, z: usize) -> char {
    match sheet.get_cell(x, y, z) {
        Some(cell) => {
            if cell.shadowed { return 's'; }
            match cell.adjust {
                Adjust::Right => 'r',
                Adjust::Center => 'c',
                _ => 'l', // Left and AutoAdjust
            }
        }
        None => 'l',
    }
}

fn get_font_modifier(sheet: &Sheet, x: usize, y: usize, z: usize) -> &'static str {
    match sheet.get_cell(x, y, z) {
        Some(cell) => {
            if cell.bold { "fB " }
            else if cell.underline { "fI " } // tbl uses italic for underline emphasis
            else { "" }
        }
        None => "",
    }
}

fn is_shadowed(sheet: &Sheet, x: usize, y: usize, z: usize) -> bool {
    sheet.get_cell(x, y, z).is_some_and(|c| c.shadowed)
}

/// Save a sheet to TBL (troff tbl) format
pub fn save_tbl(
    sheet: &Sheet,
    name: &str,
    body: bool,
    x1: usize,
    y1: usize,
    z1: usize,
    x2: usize,
    y2: usize,
    z2: usize,
) -> Result<usize> {
    let mut count = 0;

    // Check for shadowed cells in first column
    for z in z1..=z2 {
        for y in y1..=y2 {
            if is_shadowed(sheet, x1, y, z) {
                anyhow::bail!("Shadowed cells in first column");
            }
        }
    }

    if body {
        // Body mode: one file per sheet layer
        for z in z1..=z2 {
            let fullname = format!("{}.{}", name, z);
            let mut fp = std::fs::File::create(&fullname)?;
            write_tbl_table(&mut fp, sheet, x1, y1, x2, y2, z, &mut count)?;
        }
    } else {
        let mut fp = std::fs::File::create(name)?;
        for z in z1..=z2 {
            write_tbl_table(&mut fp, sheet, x1, y1, x2, y2, z, &mut count)?;
        }
    }

    Ok(count)
}

fn write_tbl_table(
    fp: &mut impl Write,
    sheet: &Sheet,
    x1: usize, y1: usize, x2: usize, y2: usize, z: usize,
    count: &mut usize,
) -> Result<()> {
    writeln!(fp, ".TS")?;
    writeln!(fp, "tab(\\t);")?;

    for y in y1..=y2 {
        if y > y1 {
            // Use .T& for subsequent format lines
            writeln!(fp, ".T&")?;
        }

        // Write format line for this row
        let mut fmt_parts = Vec::new();
        let mut x = x1;
        while x <= x2 {
            let fc = get_format_char(sheet, x, y, z);
            let fm = get_font_modifier(sheet, x, y, z);
            fmt_parts.push(format!("{}{}", fc, fm));
            x += 1;
        }
        writeln!(fp, "{}.", fmt_parts.join(" "))?;

        // Write data line
        let mut first = true;
        x = x1;
        while x <= x2 {
            if !first {
                write!(fp, "\t")?;
            }
            first = false;

            if !is_shadowed(sheet, x, y, z) {
                let val = format_value(sheet, x, y, z);
                write!(fp, "{}", escape_tbl(&val))?;
                *count += 1;
            }
            x += 1;
        }
        writeln!(fp)?;
    }

    writeln!(fp, ".TE")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::Token;
    use std::io::Read;

    #[test]
    fn test_tbl_export() {
        let mut sheet = Sheet::new();
        sheet.putcont(0, 0, 0, vec![Token::Integer(42)]);
        sheet.putcont(1, 0, 0, vec![Token::String("hello".to_string())]);
        {
            let cell = sheet.get_or_create_cell(0, 0, 0);
            cell.adjust = Adjust::Right;
            cell.bold = true;
        }
        sheet.update();

        let tmpfile = "/tmp/teapot_test_tbl.tbl";
        save_tbl(&sheet, tmpfile, false, 0, 0, 0, 1, 0, 0).unwrap();

        let mut content = String::new();
        std::fs::File::open(tmpfile).unwrap().read_to_string(&mut content).unwrap();

        assert!(content.contains(".TS"), "missing .TS");
        assert!(content.contains(".TE"), "missing .TE");
        assert!(content.contains("42"), "missing value 42");
        assert!(content.contains("hello"), "missing value hello");
        assert!(content.contains("rfB"), "missing right-bold format");

        std::fs::remove_file(tmpfile).ok();
    }

    #[test]
    fn test_tbl_body_mode() {
        let mut sheet = Sheet::new();
        sheet.putcont(0, 0, 0, vec![Token::Integer(1)]);
        sheet.update();

        let tmpfile = "/tmp/teapot_test_tbl_body";
        save_tbl(&sheet, tmpfile, true, 0, 0, 0, 0, 0, 0).unwrap();

        let body_file = format!("{}.0", tmpfile);
        let mut content = String::new();
        std::fs::File::open(&body_file).unwrap().read_to_string(&mut content).unwrap();
        assert!(content.contains(".TS"), "missing .TS in body mode");
        assert!(content.contains("1"), "missing value");

        std::fs::remove_file(&body_file).ok();
    }

    #[test]
    fn test_tbl_escaping() {
        assert_eq!(escape_tbl("normal"), "normal");
        assert_eq!(escape_tbl("back\\slash"), "back\\eslash");
        assert_eq!(escape_tbl("under_score"), "under\\&_score");
        assert_eq!(escape_tbl(".leading"), "\\&.leading");
        assert_eq!(escape_tbl("'quote"), "\\&'quote");
        assert_eq!(escape_tbl("-"), "\\-");
        assert_eq!(escape_tbl("--"), "--");
    }
}
