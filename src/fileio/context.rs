//! ConTeXt file format handler

use anyhow::Result;
use std::io::Write;
use crate::sheet::{Adjust, Sheet};
use crate::token::Token;

fn escape_context(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '%' | '$' | '&' | '#' | '_' | '{' | '}' | '~' | '^' => {
                out.push('\\');
                out.push(c);
            }
            '\\' => out.push_str("\\backslash "),
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

fn get_adjust(sheet: &Sheet, x: usize, y: usize, z: usize) -> Adjust {
    sheet.get_cell(x, y, z)
        .map(|c| if c.adjust == Adjust::AutoAdjust { Adjust::Left } else { c.adjust })
        .unwrap_or(Adjust::Left)
}

fn is_shadowed(sheet: &Sheet, x: usize, y: usize, z: usize) -> bool {
    sheet.get_cell(x, y, z).map_or(false, |c| c.shadowed)
}

fn is_transparent(sheet: &Sheet, x: usize, y: usize, z: usize) -> bool {
    sheet.get_cell(x, y, z).map_or(false, |c| c.transparent)
}

/// Save a sheet to a ConTeXt file
pub fn save_context(
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

    for z in z1..=z2 {
        for y in y1..=y2 {
            if is_shadowed(sheet, x1, y, z) {
                anyhow::bail!("Shadowed cells in first column");
            }
        }
    }

    if body {
        for z in z1..=z2 {
            let fullname = format!("{}.{}", name, z);
            let mut fp = std::fs::File::create(&fullname)?;
            write_context_table(&mut fp, sheet, x1, y1, x2, y2, z, &mut count)?;
        }
    } else {
        let mut fp = std::fs::File::create(name)?;
        writeln!(fp, "\\starttext")?;
        for z in z1..=z2 {
            if z > z1 {
                writeln!(fp, "\\page")?;
            }
            write_context_table(&mut fp, sheet, x1, y1, x2, y2, z, &mut count)?;
        }
        writeln!(fp, "\\stoptext")?;
    }

    Ok(count)
}

fn write_context_table(
    fp: &mut impl Write,
    sheet: &Sheet,
    x1: usize, y1: usize, x2: usize, y2: usize, z: usize,
    count: &mut usize,
) -> Result<()> {
    // Column spec
    write!(fp, "\\starttable[")?;
    for _ in x1..=x2 {
        write!(fp, "|l")?;
    }
    writeln!(fp, "|]")?;

    for y in y1..=y2 {
        let mut x = x1;
        while x <= x2 {
            if x > x1 {
                write!(fp, "\\NC")?;
            }

            let mut multicols = 1;
            while x + multicols <= x2 && is_shadowed(sheet, x + multicols, y, z) {
                multicols += 1;
            }

            if multicols > 1 {
                write!(fp, "\\use{{{}}}", multicols)?;
            }

            match get_adjust(sheet, x, y, z) {
                Adjust::Left | Adjust::AutoAdjust => write!(fp, "\\JustLeft ")?,
                Adjust::Right => write!(fp, "\\JustRight ")?,
                Adjust::Center => write!(fp, "\\JustCenter ")?,
            }

            let val = format_value(sheet, x, y, z);
            if is_transparent(sheet, x, y, z) {
                write!(fp, "{}", val)?;
            } else {
                write!(fp, "{}", escape_context(&val))?;
            }

            x += multicols;
            *count += 1;
        }
        if y < y2 {
            writeln!(fp, "\\MR")?;
        } else {
            writeln!(fp, "\n\\stoptable")?;
        }
    }
    Ok(())
}
