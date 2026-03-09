use anyhow::{Result, Context};
use clap::{Parser, ArgAction};
use std::path::{Path, PathBuf};
use std::io::{self, BufRead};
use teapotlib::{
    Sheet,
    display::display_main,
    fileio::{load_xdr, load_port, load_sc, load_wk1, load_csv},
};

#[derive(Parser, Debug)]
#[command(author, version, about = "Table Editor And Planner, Or: Teapot")]
struct Args {
    /// Use ASCII file format as default
    #[arg(short = 'a', long, action = ArgAction::SetTrue)]
    ascii: bool,

    /// Batch mode (read commands from stdin)
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

/// Load a file based on its extension
fn load_file(sheet: &mut Sheet, path: &Path, use_xdr: bool) -> Result<()> {
    let filename = path.to_str().unwrap_or("");
    if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
        match extension.to_lowercase().as_str() {
            "tpa" => load_port(sheet, filename)?,
            "sc" => load_sc(sheet, filename)?,
            "wk1" => load_wk1(sheet, filename)?,
            "csv" => load_csv(sheet, filename)?,
            _ => {
                if use_xdr {
                    load_xdr(sheet, filename)?;
                } else {
                    load_port(sheet, filename)?;
                }
            }
        }
    } else if use_xdr {
        load_xdr(sheet, filename)?;
    } else {
        load_port(sheet, filename)?;
    }
    Ok(())
}

/// Process batch mode commands from stdin
fn process_batch(sheet: &mut Sheet) -> Result<()> {
    let stdin = io::stdin();
    for line_result in stdin.lock().lines() {
        let line = line_result?;
        let line = line.trim();
        if line.is_empty() { continue; }

        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        let cmd = parts[0];
        let arg = parts.get(1).map(|s| s.trim()).unwrap_or("");

        match cmd {
            "goto" => {
                let coords: Vec<&str> = arg.split(',').collect();
                if coords.len() >= 2 {
                    if let (Ok(x), Ok(y)) = (coords[0].trim().parse(), coords[1].trim().parse()) {
                        sheet.cur_x = x;
                        sheet.cur_y = y;
                        if coords.len() > 2 {
                            if let Ok(z) = coords[2].trim().parse() {
                                sheet.cur_z = z;
                            }
                        }
                    }
                }
            }
            "save-csv" => {
                teapotlib::fileio::save_csv(sheet, arg, ',',
                    0, 0, 0, sheet.dim_x.saturating_sub(1), sheet.dim_y.saturating_sub(1), 0)?;
            }
            "load-csv" => {
                load_csv(sheet, arg)?;
            }
            _ => {
                eprintln!("Unknown batch command: {}", cmd);
            }
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut sheet = Sheet::new();

    if let Some(file_path) = &args.file {
        let name = file_path.to_str().unwrap_or("").to_string();
        sheet.name = Some(name.clone());
        load_file(&mut sheet, file_path, !args.ascii)
            .with_context(|| format!("Failed to load file: {}", name))?;
    }

    if args.batch {
        process_batch(&mut sheet)?;
    } else {
        display_main(&mut sheet);
    }

    Ok(())
}
