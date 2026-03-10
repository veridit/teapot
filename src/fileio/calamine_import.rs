//! Import from xlsx, ods, and xls formats using the calamine crate

use anyhow::{bail, Result};
use calamine::{open_workbook_auto, Data, Reader};
use crate::sheet::{Adjust, Sheet};
use crate::token::Token;

/// Load a sheet from an xlsx, ods, or xls file
pub fn load_spreadsheet(sheet: &mut Sheet, filename: &str) -> Result<()> {
    let mut workbook = open_workbook_auto(filename)
        .map_err(|e| anyhow::anyhow!("Failed to open {}: {}", filename, e))?;

    let sheet_names = workbook.sheet_names().to_vec();
    if sheet_names.is_empty() {
        bail!("No sheets found in {}", filename);
    }

    for (z, sheet_name) in sheet_names.iter().enumerate() {
        let range = match workbook.worksheet_range(sheet_name) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Warning: could not read sheet '{}': {}", sheet_name, e);
                continue;
            }
        };

        let (height, width) = range.get_size();
        if width == 0 || height == 0 {
            continue;
        }

        for (row_idx, row) in range.rows().enumerate() {
            for (col_idx, cell) in row.iter().enumerate() {
                match cell {
                    Data::Empty => {}
                    Data::String(s) => {
                        let c = sheet.get_or_create_cell(col_idx, row_idx, z);
                        c.adjust = Adjust::Left;
                        sheet.putcont(col_idx, row_idx, z, vec![Token::String(s.clone())]);
                    }
                    Data::Float(f) => {
                        if *f == (*f as i64) as f64 && f.abs() < i64::MAX as f64 {
                            sheet.putcont(col_idx, row_idx, z, vec![Token::Integer(*f as i64)]);
                        } else {
                            sheet.putcont(col_idx, row_idx, z, vec![Token::Float(*f)]);
                        }
                    }
                    Data::Int(i) => {
                        sheet.putcont(col_idx, row_idx, z, vec![Token::Integer(*i)]);
                    }
                    Data::Bool(b) => {
                        sheet.putcont(col_idx, row_idx, z, vec![Token::Integer(if *b { 1 } else { 0 })]);
                    }
                    Data::Error(e) => {
                        sheet.putcont(col_idx, row_idx, z, vec![Token::String(format!("{:?}", e))]);
                    }
                    Data::DateTime(dt) => {
                        let f = dt.as_f64();
                        sheet.putcont(col_idx, row_idx, z, vec![Token::Float(f)]);
                    }
                    Data::DateTimeIso(s) | Data::DurationIso(s) => {
                        sheet.putcont(col_idx, row_idx, z, vec![Token::String(s.clone())]);
                    }
                }
            }
        }

        if width > sheet.dim_x {
            sheet.dim_x = width;
        }
        if height > sheet.dim_y {
            sheet.dim_y = height;
        }
    }

    if sheet_names.len() > sheet.dim_z {
        sheet.dim_z = sheet_names.len();
    }

    sheet.changed = false;
    sheet.cachelabels();
    sheet.update();
    Ok(())
}
