//! WK1 file format handler

use anyhow::Result;
use crate::sheet::Sheet;

/// Load a sheet from a WK1 file
pub fn load_wk1(sheet: &mut Sheet, name: &str) -> Result<()> {
    // TODO: Implement WK1 loading
    println!("Loading WK1 file: {}", name);
    Ok(())
}
