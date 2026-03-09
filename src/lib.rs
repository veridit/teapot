//! Teapot library - Table Editor And Planner, Or: Teapot
//! 
//! This is the core library for the Teapot spreadsheet application

// Re-export modules
pub mod sheet;
pub mod token;
pub mod parser;
pub mod eval;
pub mod scanner;
pub mod fileio;
pub mod display;
pub mod utils;

// Create the Sheet struct
pub use sheet::Sheet;
