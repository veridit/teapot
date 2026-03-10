//! WK1 (Lotus 1-2-3) file format handler

use anyhow::{bail, Result};
use std::io::Read;
use crate::sheet::Sheet;
use crate::token::Token;

/// Read a 16-bit little-endian integer from two bytes
fn it(s: &[u8]) -> usize {
    (s[0] as usize) | ((s[1] as usize) << 8)
}

/// Read a 16-bit little-endian signed integer
fn it_signed(s: &[u8]) -> i64 {
    let v = (s[0] as u16) | ((s[1] as u16) << 8);
    v as i16 as i64
}

/// Convert 8 bytes of IEEE 754 double (little-endian) to f64
fn dbl(s: &[u8]) -> f64 {
    let bytes: [u8; 8] = [s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]];
    f64::from_le_bytes(bytes)
}

/// Apply format byte to a cell
fn apply_format(format_byte: u8, sheet: &mut Sheet, x: usize, y: usize, z: usize) {
    let cell = sheet.get_or_create_cell(x, y, z);
    let fmt_type = (format_byte & 0x70) >> 4;
    let precision = (format_byte & 0x0f) as i32;

    match fmt_type {
        0 => {
            // Fixed with given precision
            cell.precision = precision;
            cell.scientific = false;
        }
        1 => {
            // Scientific with given precision
            cell.precision = precision;
            cell.scientific = true;
        }
        2..=4 => {
            // Currency, percent, comma with given precision
            cell.precision = precision;
        }
        _ => {} // Special formats, ignore
    }

    if format_byte & 0x80 != 0 {
        cell.locked = true;
    }
}

/// Load a sheet from a WK1 (Lotus 1-2-3) file
pub fn load_wk1(sheet: &mut Sheet, name: &str) -> Result<()> {
    let mut fp = std::fs::File::open(name)?;
    let mut found_bof = false;
    let mut found_eof = false;

    loop {
        // Read 4-byte record header
        let mut header = [0u8; 4];
        match fp.read_exact(&mut header) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        }

        let record_type = (header[0] as u16) | ((header[1] as u16) << 8);
        let body_len = (header[2] as usize) | ((header[3] as usize) << 8);

        // Read body
        let mut body = vec![0u8; body_len];
        if body_len > 0 {
            fp.read_exact(&mut body)?;
        }

        match record_type {
            // BOF - Beginning of file
            0x0000 => {
                if body_len != 2 {
                    bail!("Invalid BOF record body length");
                }
                if !found_bof {
                    found_bof = true;
                }
            }
            // EOF - End of file
            0x0001 => {
                found_eof = true;
                break;
            }
            // RANGE - Active worksheet range
            0x0006 => {
                if body_len == 8 {
                    let max_x = it(&body[4..6]);
                    let max_y = it(&body[6..8]);
                    if max_x > 0 { sheet.dim_x = max_x; }
                    if max_y > 0 { sheet.dim_y = max_y; }
                }
            }
            // BLANK - Blank cell
            0x000C => {
                if body_len == 5 {
                    let x = it(&body[1..3]);
                    let y = it(&body[3..5]);
                    apply_format(body[0], sheet, x, y, 0);
                }
            }
            // INTEGER - Integer number cell
            0x000D => {
                if body_len == 7 {
                    let x = it(&body[1..3]);
                    let y = it(&body[3..5]);
                    let val = it_signed(&body[5..7]);
                    sheet.putcont(x, y, 0, vec![Token::Integer(val)]);
                    apply_format(body[0], sheet, x, y, 0);
                }
            }
            // NUMBER - Floating point number
            0x000E => {
                if body_len == 13 {
                    let x = it(&body[1..3]);
                    let y = it(&body[3..5]);
                    let val = dbl(&body[5..13]);
                    sheet.putcont(x, y, 0, vec![Token::Float(val)]);
                    apply_format(body[0], sheet, x, y, 0);
                }
            }
            // LABEL - Label cell
            0x000F => {
                if body_len >= 6 {
                    let x = it(&body[1..3]);
                    let y = it(&body[3..5]);
                    // Label prefix byte at body[5]: ' = left, " = right, ^ = center
                    let text_start = 6;
                    // Find null terminator
                    let text_end = body[text_start..].iter()
                        .position(|&b| b == 0)
                        .map(|p| text_start + p)
                        .unwrap_or(body_len);
                    let text = String::from_utf8_lossy(&body[text_start..text_end]).to_string();
                    sheet.putcont(x, y, 0, vec![Token::String(text)]);
                    apply_format(body[0], sheet, x, y, 0);
                }
            }
            // FORMULA - Formula cell (store the cached value, skip RPN)
            0x0010 => {
                if body_len > 15 {
                    let x = it(&body[1..3]);
                    let y = it(&body[3..5]);
                    let val = dbl(&body[5..13]);
                    // Store the cached value as a float constant
                    sheet.putcont(x, y, 0, vec![Token::Float(val)]);
                    apply_format(body[0], sheet, x, y, 0);
                }
            }
            // STRING - Value of string formula
            0x0033 => {
                if body_len >= 6 {
                    let x = it(&body[1..3]);
                    let y = it(&body[3..5]);
                    let text_end = body[5..].iter()
                        .position(|&b| b == 0)
                        .map(|p| 5 + p)
                        .unwrap_or(body_len);
                    let text = String::from_utf8_lossy(&body[5..text_end]).to_string();
                    sheet.putcont(x, y, 0, vec![Token::String(text)]);
                    apply_format(body[0], sheet, x, y, 0);
                }
            }
            // All other record types: silently skip
            _ => {}
        }

        if !found_bof && record_type != 0x0000 {
            bail!("This is not a WK1 file");
        }
    }

    if !found_eof {
        eprintln!("Warning: WK1 file appears truncated");
    }

    sheet.changed = false;
    sheet.cachelabels();
    sheet.update();
    Ok(())
}
