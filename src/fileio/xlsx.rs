//! XLSX export using rust_xlsxwriter

use anyhow::Result;
use rust_xlsxwriter::{Format, Workbook};
use crate::sheet::{Adjust, Sheet};
use crate::token::Token;

/// Save a sheet as an XLSX file
pub fn save_xlsx(sheet: &Sheet, filename: &str) -> Result<usize> {
    let mut workbook = Workbook::new();
    let mut count = 0;

    // Create one worksheet per z-layer
    let max_z = sheet.dim_z.max(1) - 1;

    for z in 0..=max_z {
        let worksheet = workbook.add_worksheet();
        if z == 0 {
            worksheet.set_name("Sheet1")?;
        } else {
            worksheet.set_name(format!("Sheet{}", z + 1))?;
        }

        // Set column widths
        for x in 0..sheet.dim_x {
            let w = sheet.column_width(x, z);
            // Excel width is in character units; Teapot width is similar
            worksheet.set_column_width(x as u16, w as f64)?;
        }

        // Write cells
        for y in 0..sheet.dim_y {
            for x in 0..sheet.dim_x {
                if let Some(cell) = sheet.get_cell(x, y, z) {
                    let mut fmt = Format::new();

                    // Alignment
                    match cell.adjust {
                        Adjust::Left => {
                            fmt = fmt.set_align(rust_xlsxwriter::FormatAlign::Left);
                        }
                        Adjust::Right => {
                            fmt = fmt.set_align(rust_xlsxwriter::FormatAlign::Right);
                        }
                        Adjust::Center => {
                            fmt = fmt.set_align(rust_xlsxwriter::FormatAlign::Center);
                        }
                        _ => {}
                    }

                    // Bold
                    if cell.bold {
                        fmt = fmt.set_bold();
                    }

                    // Underline
                    if cell.underline {
                        fmt = fmt.set_underline(rust_xlsxwriter::FormatUnderline::Single);
                    }

                    // Number format based on precision
                    if cell.precision >= 0 {
                        let num_fmt = if cell.scientific {
                            format!("0.{}E+00", "0".repeat(cell.precision as usize))
                        } else {
                            if cell.precision == 0 {
                                "0".to_string()
                            } else {
                                format!("0.{}", "0".repeat(cell.precision as usize))
                            }
                        };
                        fmt = fmt.set_num_format(&num_fmt);
                    }

                    let row = y as u32;
                    let col = x as u16;

                    match &cell.value {
                        Token::Integer(i) => {
                            worksheet.write_number_with_format(row, col, *i as f64, &fmt)?;
                            count += 1;
                        }
                        Token::Float(f) => {
                            worksheet.write_number_with_format(row, col, *f, &fmt)?;
                            count += 1;
                        }
                        Token::String(s) => {
                            worksheet.write_string_with_format(row, col, s, &fmt)?;
                            count += 1;
                        }
                        Token::Empty => {
                            // Write format only if there's formatting
                            if cell.bold || cell.underline || cell.adjust != Adjust::Right {
                                worksheet.write_string_with_format(row, col, "", &fmt)?;
                            }
                        }
                        _ => {
                            // For other token types, write as string representation
                            let s = format!("{:?}", cell.value);
                            worksheet.write_string_with_format(row, col, &s, &fmt)?;
                            count += 1;
                        }
                    }
                }
            }
        }
    }

    workbook.save(filename)?;
    Ok(count)
}
