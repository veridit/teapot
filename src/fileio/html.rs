//! HTML file format handler

use anyhow::Result;
use crate::sheet::Sheet;

/// Save a sheet to an HTML file
pub fn save_html(
    _sheet: &Sheet,
    name: &str,
    _body: bool,
    _x1: usize,
    _y1: usize,
    _z1: usize,
    _x2: usize,
    _y2: usize,
    _z2: usize
) -> Result<usize> {
    // TODO: Implement HTML saving
    println!("Saving HTML file: {}", name);
    Ok(0) // Return number of cells written
}
