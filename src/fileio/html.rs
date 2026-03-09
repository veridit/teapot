//! HTML file format handler

use anyhow::Result;
use crate::sheet::Sheet;

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
    z2: usize
) -> Result<usize> {
    // TODO: Implement HTML saving
    println!("Saving HTML file: {}", name);
    Ok(0) // Return number of cells written
}
