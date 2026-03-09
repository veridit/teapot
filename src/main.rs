//! Teapot - Table Editor And Planner, Or: Teapot
//! 
//! A terminal-based spreadsheet application

use anyhow::{Result, Context};
use clap::{Parser, ArgAction};
use std::path::{Path, PathBuf};
use std::io::{self, BufRead};
use teapotlib::{
    Sheet,
    display::{display_init, display_main, display_end},
    fileio::{load_xdr, load_port, load_sc, load_wk1, load_csv},
    utils::find_help_file
};

/// Command line arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Use ASCII file format as default
    #[arg(short = 'a', long, action = ArgAction::SetTrue)]
    ascii: bool,

    /// Batch mode
    #[arg(short = 'b', long, action = ArgAction::SetTrue)]
    batch: bool,

    /// Display strings in quotes
    #[arg(short = 'q', long, action = ArgAction::SetTrue)]
    quote: bool,

    /// Hide row/column headers
    #[arg(short = 'H', long, action = ArgAction::SetTrue)]
    hide_headers: bool,

    /// Redraw more often
    #[arg(short = 'r', long, action = ArgAction::SetTrue)]
    redraw: bool,

    /// Set decimal precision
    #[arg(short = 'p', long, value_name = "DIGITS", default_value = "6")]
    precision: usize,

    /// Input file
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,
}

/// Global application state
struct AppState {
    /// Path to help file
    help_file: PathBuf,
    /// Batch mode flag
    batch: bool,
    /// Batch line number
    batch_ln: usize,
    /// Default precision
    def_precision: usize,
    /// Quote strings flag
    quote: bool,
    /// Show headers flag
    header: bool,
    /// Use XDR format flag
    use_xdr: bool,
}

impl AppState {
    fn new(args: &Args) -> Self {
        AppState {
            help_file: find_help_file(&std::env::args().next().unwrap_or_default()),
            batch: args.batch,
            batch_ln: 0,
            def_precision: args.precision,
            quote: args.quote,
            header: !args.hide_headers,
            use_xdr: !args.ascii,
        }
    }
}

/// Process batch mode commands
fn process_batch(sheet: &mut Sheet, state: &mut AppState) -> Result<()> {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    
    while let Some(Ok(line)) = lines.next() {
        state.batch_ln += 1;
        
        // Skip empty lines
        if line.trim().is_empty() {
            continue;
        }
        
        // Parse command and arguments
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        let cmd = parts[0];
        let arg = parts.get(1).unwrap_or(&"").trim();
        
        match cmd {
            "goto" => {
                println!("Batch command: goto {}", arg);
                // TODO: Implement goto
            },
            "from" => {
                println!("Batch command: from {}", arg);
                // TODO: Implement from
            },
            "to" => {
                println!("Batch command: to {}", arg);
                // TODO: Implement to
            },
            "save-tbl" => {
                println!("Batch command: save-tbl {}", arg);
                // TODO: Implement save-tbl
            },
            "save-latex" => {
                println!("Batch command: save-latex {}", arg);
                // TODO: Implement save-latex
            },
            "save-context" => {
                println!("Batch command: save-context {}", arg);
                // TODO: Implement save-context
            },
            "save-csv" => {
                println!("Batch command: save-csv {}", arg);
                // TODO: Implement save-csv
            },
            "save-html" => {
                println!("Batch command: save-html {}", arg);
                // TODO: Implement save-html
            },
            "load-csv" => {
                println!("Batch command: load-csv {}", arg);
                // TODO: Implement load-csv
            },
            "sort-x" => {
                println!("Batch command: sort-x {}", arg);
                // TODO: Implement sort-x
            },
            "sort-y" => {
                println!("Batch command: sort-y {}", arg);
                // TODO: Implement sort-y
            },
            "sort-z" => {
                println!("Batch command: sort-z {}", arg);
                // TODO: Implement sort-z
            },
            _ => {
                println!("Unknown batch command: {}", cmd);
            }
        }
    }
    
    Ok(())
}

/// Load a file based on its extension
fn load_file(sheet: &mut Sheet, path: &Path, use_xdr: bool) -> Result<()> {
    if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
        match extension.to_lowercase().as_str() {
            "tpa" => load_port(sheet, path.to_str().unwrap())?,
            "sc" => load_sc(sheet, path.to_str().unwrap())?,
            "wk1" => load_wk1(sheet, path.to_str().unwrap())?,
            "csv" => load_csv(sheet, path.to_str().unwrap())?,
            _ => {
                // Default to XDR or ASCII format
                if use_xdr {
                    load_xdr(sheet, path.to_str().unwrap())?;
                } else {
                    load_port(sheet, path.to_str().unwrap())?;
                }
            }
        }
    } else {
        // No extension, use default format
        if use_xdr {
            load_xdr(sheet, path.to_str().unwrap())?;
        } else {
            load_port(sheet, path.to_str().unwrap())?;
        }
    }
    
    Ok(())
}

fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();
    
    // Initialize application state
    let mut app_state = AppState::new(&args);
    
    // Create a new sheet
    let mut sheet = Sheet::new();
    
    // Set sheet name and load file if provided
    if let Some(file_path) = &args.file {
        if let Some(file_name) = file_path.to_str() {
            sheet.name = Some(file_name.to_string());
            
            // Load the file based on extension
            load_file(&mut sheet, file_path, app_state.use_xdr)
                .with_context(|| format!("Failed to load file: {}", file_name))?;
        }
    }
    
    if app_state.batch {
        // Process batch mode
        process_batch(&mut sheet, &mut app_state)?;
    } else {
        // Interactive mode
        display_init(&sheet, args.redraw);
        display_main(&mut sheet);
        // display_end() is now called inside display_main
    }
    
    Ok(())
}
