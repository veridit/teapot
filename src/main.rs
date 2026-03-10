use anyhow::{Result, Context};
use clap::{Parser, ArgAction};
use std::path::PathBuf;
use std::io::{self, BufRead};
use regex::Regex;
use teapotlib::{
    Sheet,
    display::display_main,
    scanner,
    fileio::{self, load_file, load_csv},
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

/// Parse "x,y[,z]" coordinate string
fn parse_coords(s: &str) -> Option<(usize, usize, usize)> {
    let coords: Vec<&str> = s.split(',').collect();
    if coords.len() >= 2 {
        let x = coords[0].trim().parse().ok()?;
        let y = coords[1].trim().parse().ok()?;
        let z = coords.get(2).and_then(|s| s.trim().parse().ok()).unwrap_or(0);
        Some((x, y, z))
    } else {
        None
    }
}

/// Process batch mode commands from stdin
fn process_batch(sheet: &mut Sheet) -> Result<()> {
    let stdin = io::stdin();
    for line_result in stdin.lock().lines() {
        let line = line_result?;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }

        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        let cmd = parts[0];
        let arg = parts.get(1).map(|s| s.trim()).unwrap_or("");

        match cmd {
            // Navigation
            "goto" => {
                if let Some((x, y, z)) = parse_coords(arg) {
                    sheet.cur_x = x;
                    sheet.cur_y = y;
                    sheet.cur_z = z;
                } else {
                    eprintln!("goto: expected x,y[,z]");
                }
            }
            "from" => {
                if let Some((x, y, z)) = parse_coords(arg) {
                    sheet.mark1_x = Some(x);
                    sheet.mark1_y = Some(y);
                    sheet.mark1_z = Some(z);
                } else {
                    eprintln!("from: expected x,y[,z]");
                }
            }
            "to" => {
                if let Some((x, y, z)) = parse_coords(arg) {
                    sheet.mark2_x = Some(x);
                    sheet.mark2_y = Some(y);
                    sheet.mark2_z = Some(z);
                } else {
                    eprintln!("to: expected x,y[,z]");
                }
            }

            // Cell editing
            "set" => {
                // set x,y[,z] expression
                let set_parts: Vec<&str> = arg.splitn(2, ' ').collect();
                if set_parts.len() == 2 {
                    if let Some((x, y, z)) = parse_coords(set_parts[0]) {
                        match scanner::scan(set_parts[1]) {
                            Ok(tokens) => {
                                sheet.putcont(x, y, z, tokens);
                                sheet.update();
                            }
                            Err(e) => eprintln!("set: parse error: {}", e),
                        }
                    } else {
                        eprintln!("set: expected x,y[,z] expression");
                    }
                } else {
                    eprintln!("set: expected x,y[,z] expression");
                }
            }
            "print" => {
                if let Some((x, y, z)) = parse_coords(arg) {
                    if let Some(cell) = sheet.get_cell(x, y, z) {
                        println!("{}", cell.value);
                    } else {
                        println!();
                    }
                } else {
                    eprintln!("print: expected x,y[,z]");
                }
            }

            // Formatting
            "width" => {
                let parts: Vec<&str> = arg.split_whitespace().collect();
                if parts.len() == 2 {
                    if let (Ok(col), Ok(w)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
                        sheet.set_width(col, sheet.cur_z, w);
                    } else {
                        eprintln!("width: expected col width");
                    }
                } else {
                    eprintln!("width: expected col width");
                }
            }
            "precision" => {
                let parts: Vec<&str> = arg.split_whitespace().collect();
                if parts.len() == 2 {
                    if let (Ok(col), Ok(prec)) = (parts[0].parse::<usize>(), parts[1].parse::<i32>()) {
                        for y in 0..sheet.dim_y {
                            let cell = sheet.get_or_create_cell(col, y, sheet.cur_z);
                            cell.precision = prec;
                        }
                    } else {
                        eprintln!("precision: expected col precision");
                    }
                } else {
                    eprintln!("precision: expected col precision");
                }
            }

            // Sort
            "sort" | "sort-x" => {
                if let Some((x1, y1, z1, x2, y2, z2)) = sheet.get_mark_range() {
                    let parts: Vec<&str> = arg.split_whitespace().collect();
                    let sort_col = parts.first()
                        .and_then(|s| s.parse::<usize>().ok())
                        .unwrap_or(sheet.cur_x);
                    let ascending = parts.get(1)
                        .map(|s| !s.starts_with('d'))
                        .unwrap_or(true);
                    sheet.sort_block(x1, y1, z1, x2, y2, z2, sort_col, ascending);
                } else {
                    eprintln!("sort: no block marked (use from/to first)");
                }
            }
            "sort-y" => {
                if let Some((x1, y1, z1, x2, y2, z2)) = sheet.get_mark_range() {
                    let parts: Vec<&str> = arg.split_whitespace().collect();
                    let sort_row = parts.first()
                        .and_then(|s| s.parse::<usize>().ok())
                        .unwrap_or(sheet.cur_y);
                    let ascending = parts.get(1)
                        .map(|s| !s.starts_with('d'))
                        .unwrap_or(true);
                    sheet.sort_block_y(x1, y1, z1, x2, y2, z2, sort_row, ascending);
                } else {
                    eprintln!("sort-y: no block marked (use from/to first)");
                }
            }
            "sort-z" => {
                if let Some((x1, y1, z1, x2, y2, z2)) = sheet.get_mark_range() {
                    let parts: Vec<&str> = arg.split_whitespace().collect();
                    let (sort_x, sort_y) = if let Some(coord) = parts.first() {
                        let coords: Vec<&str> = coord.split(',').collect();
                        (
                            coords.first().and_then(|s| s.parse().ok()).unwrap_or(sheet.cur_x),
                            coords.get(1).and_then(|s| s.parse().ok()).unwrap_or(sheet.cur_y),
                        )
                    } else {
                        (sheet.cur_x, sheet.cur_y)
                    };
                    let ascending = parts.get(1)
                        .map(|s| !s.starts_with('d'))
                        .unwrap_or(true);
                    sheet.sort_block_z(x1, y1, z1, x2, y2, z2, sort_x, sort_y, ascending);
                } else {
                    eprintln!("sort-z: no block marked (use from/to first)");
                }
            }
            // Mirror
            "mirror-x" => {
                if let Some((x1, y1, z1, x2, y2, z2)) = sheet.get_mark_range() {
                    sheet.mirror_block(x1, y1, z1, x2, y2, z2, teapotlib::sheet::Direction::X);
                } else {
                    eprintln!("mirror-x: no block marked");
                }
            }
            "mirror-y" => {
                if let Some((x1, y1, z1, x2, y2, z2)) = sheet.get_mark_range() {
                    sheet.mirror_block(x1, y1, z1, x2, y2, z2, teapotlib::sheet::Direction::Y);
                } else {
                    eprintln!("mirror-y: no block marked");
                }
            }
            "mirror-z" => {
                if let Some((x1, y1, z1, x2, y2, z2)) = sheet.get_mark_range() {
                    sheet.mirror_block(x1, y1, z1, x2, y2, z2, teapotlib::sheet::Direction::Z);
                } else {
                    eprintln!("mirror-z: no block marked");
                }
            }
            // Fill
            "fill" => {
                if let Some((x1, y1, z1, x2, y2, z2)) = sheet.get_mark_range() {
                    let parts: Vec<&str> = arg.split_whitespace().collect();
                    let cols = parts.first().and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
                    let rows = parts.get(1).and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
                    let layers = parts.get(2).and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
                    sheet.fill_block(x1, y1, z1, x2, y2, z2, cols, rows, layers);
                } else {
                    eprintln!("fill: no block marked");
                }
            }
            // Clock
            "clock" => {
                let (x, y, z) = (sheet.cur_x, sheet.cur_y, sheet.cur_z);
                let enabled = sheet.toggle_clock(x, y, z);
                eprintln!("clock {}", if enabled { "enabled" } else { "disabled" });
            }
            "clock-tick" => {
                let count = sheet.clock_tick();
                eprintln!("clock tick: {} cells", count);
            }

            // Load/save
            "load" => {
                let path = std::path::Path::new(arg);
                match load_file(sheet, path, false) {
                    Ok(()) => {}
                    Err(e) => eprintln!("load: {}", e),
                }
            }
            "load-csv" => {
                match load_csv(sheet, arg) {
                    Ok(()) => {}
                    Err(e) => eprintln!("load-csv: {}", e),
                }
            }
            "save" => {
                let filename = if arg.is_empty() {
                    sheet.name.clone().unwrap_or_else(|| "sheet.tp".to_string())
                } else {
                    arg.to_string()
                };
                if filename.ends_with(".tpz") {
                    match fileio::save_tpz(sheet, &filename) {
                        Ok(_) => { sheet.changed = false; }
                        Err(e) => eprintln!("save: {}", e),
                    }
                } else if filename.ends_with(".xlsx") {
                    match fileio::xlsx::save_xlsx(sheet, &filename) {
                        Ok(_) => { sheet.changed = false; }
                        Err(e) => eprintln!("save: {}", e),
                    }
                } else {
                    match fileio::save_port(sheet, &filename) {
                        Ok(_) => { sheet.changed = false; }
                        Err(e) => eprintln!("save: {}", e),
                    }
                }
            }
            "save-csv" => {
                let x2 = sheet.dim_x.saturating_sub(1);
                let y2 = sheet.dim_y.saturating_sub(1);
                match fileio::save_csv(sheet, arg, ',', 0, 0, 0, x2, y2, 0) {
                    Ok(_) => {}
                    Err(e) => eprintln!("save-csv: {}", e),
                }
            }
            "save-html" => {
                let (x2, y2, z2) = (sheet.dim_x.saturating_sub(1), sheet.dim_y.saturating_sub(1), sheet.dim_z.saturating_sub(1));
                match fileio::save_html(sheet, arg, false, 0, 0, 0, x2, y2, z2) {
                    Ok(_) => {}
                    Err(e) => eprintln!("save-html: {}", e),
                }
            }
            "save-latex" => {
                let (x2, y2, z2) = (sheet.dim_x.saturating_sub(1), sheet.dim_y.saturating_sub(1), sheet.dim_z.saturating_sub(1));
                match fileio::save_latex(sheet, arg, false, 0, 0, 0, x2, y2, z2) {
                    Ok(_) => {}
                    Err(e) => eprintln!("save-latex: {}", e),
                }
            }
            "save-context" => {
                let (x2, y2, z2) = (sheet.dim_x.saturating_sub(1), sheet.dim_y.saturating_sub(1), sheet.dim_z.saturating_sub(1));
                match fileio::save_context(sheet, arg, false, 0, 0, 0, x2, y2, z2) {
                    Ok(_) => {}
                    Err(e) => eprintln!("save-context: {}", e),
                }
            }
            "save-xlsx" => {
                match fileio::xlsx::save_xlsx(sheet, arg) {
                    Ok(_) => {}
                    Err(e) => eprintln!("save-xlsx: {}", e),
                }
            }
            "save-text" => {
                let x2 = sheet.dim_x.saturating_sub(1);
                let y2 = sheet.dim_y.saturating_sub(1);
                match fileio::save_text(sheet, arg, 0, 0, 0, x2, y2, 0) {
                    Ok(_) => {}
                    Err(e) => eprintln!("save-text: {}", e),
                }
            }
            "search" => {
                if arg.is_empty() {
                    eprintln!("search: expected pattern");
                } else {
                    match Regex::new(arg) {
                        Ok(re) => {
                            let results = sheet.search_cells(sheet.cur_z, &re, true);
                            for (x, y, z) in &results {
                                let val = sheet.get_cell(*x, *y, *z)
                                    .map(|c| c.value.to_string())
                                    .unwrap_or_default();
                                println!("({},{},{}) {}", x, y, z, val);
                            }
                            if results.is_empty() {
                                eprintln!("No matches for '{}'", arg);
                            }
                        }
                        Err(e) => eprintln!("search: invalid regex: {}", e),
                    }
                }
            }
            "replace" => {
                let rparts: Vec<&str> = arg.splitn(2, ' ').collect();
                if rparts.len() == 2 {
                    match Regex::new(rparts[0]) {
                        Ok(re) => {
                            let results = sheet.search_cells(sheet.cur_z, &re, true);
                            let mut count = 0;
                            for &(x, y, z) in &results {
                                if sheet.replace_cell(x, y, z, &re, rparts[1], true) {
                                    count += 1;
                                }
                            }
                            sheet.update();
                            eprintln!("Replaced {} matches", count);
                        }
                        Err(e) => eprintln!("replace: invalid regex: {}", e),
                    }
                } else {
                    eprintln!("replace: expected <pattern> <replacement>");
                }
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
