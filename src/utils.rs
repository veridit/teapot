//! Utility functions for the Teapot application

use std::path::{Path, PathBuf};

/// Find the help file based on the executable path
pub fn find_help_file(executable_path: &str) -> PathBuf {
    let exe_path = Path::new(executable_path);
    
    // Try to find the help file in various locations
    let possible_locations = vec![
        // Same directory as executable
        exe_path.with_file_name("teapot.help"),
        // ../share/teapot/
        exe_path.parent()
            .unwrap_or_else(|| Path::new(""))
            .join("../share/teapot/teapot.help"),
        // /usr/share/teapot/
        PathBuf::from("/usr/share/teapot/teapot.help"),
        // /usr/local/share/teapot/
        PathBuf::from("/usr/local/share/teapot/teapot.help"),
    ];
    
    // Return the first location that exists, or the default
    for path in possible_locations.iter() {
        if path.exists() {
            return path.to_path_buf();
        }
    }
    
    // Default to a path relative to the executable
    exe_path.with_file_name("teapot.help")
}
