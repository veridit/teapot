//! SC file format handler

use anyhow::Result;
use crate::sheet::Sheet;

/// Load a sheet from an SC file
pub fn load_sc(sheet: &mut Sheet, name: &str) -> Result<()> {
    // TODO: Implement SC loading
    println!("Loading SC file: {}", name);
    Ok(())
}
