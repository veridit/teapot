//! HTML file format handler

use anyhow::Result;
use std::io::Write;
use crate::sheet::{Adjust, Sheet};
use crate::token::Token;

fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
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

/// Save a sheet to an HTML file
pub fn save_html(
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
        // Body mode: one file per sheet
        for z in z1..=z2 {
            let fullname = format!("{}.{}", name, z);
            let mut fp = std::fs::File::create(&fullname)?;
            write_html_table(&mut fp, sheet, x1, y1, x2, y2, z, &mut count)?;
        }
    } else {
        // Single file with full HTML
        let mut fp = std::fs::File::create(name)?;
        writeln!(fp, "<html>\n<head>\n<title>\n</title>\n</head>\n<body>")?;
        for z in z1..=z2 {
            write_html_table(&mut fp, sheet, x1, y1, x2, y2, z, &mut count)?;
        }
        writeln!(fp, "</body>\n</html>")?;
    }

    Ok(count)
}

fn write_html_table(
    fp: &mut impl Write,
    sheet: &Sheet,
    x1: usize, y1: usize, x2: usize, y2: usize, z: usize,
    count: &mut usize,
) -> Result<()> {
    writeln!(fp, "<table>")?;
    for y in y1..=y2 {
        write!(fp, "<tr>")?;
        let mut x = x1;
        while x <= x2 {
            // Count colspan
            let mut multicols = 1;
            while x + multicols <= x2 && is_shadowed(sheet, x + multicols, y, z) {
                multicols += 1;
            }

            if multicols > 1 {
                write!(fp, "<td colspan={}", multicols)?;
            } else {
                write!(fp, "<td")?;
            }

            match get_adjust(sheet, x, y, z) {
                Adjust::Left | Adjust::AutoAdjust => write!(fp, " align=left>")?,
                Adjust::Right => write!(fp, " align=right>")?,
                Adjust::Center => write!(fp, " align=center>")?,
            }

            let val = format_value(sheet, x, y, z);
            if is_transparent(sheet, x, y, z) {
                write!(fp, "{}", val)?;
            } else {
                write!(fp, "{}", escape_html(&val))?;
            }
            write!(fp, "</td>")?;

            x += multicols;
            *count += 1;
        }
        writeln!(fp, "</tr>")?;
    }
    writeln!(fp, "</table>")?;
    Ok(())
}
