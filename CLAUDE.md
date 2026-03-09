# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Teapot is a terminal-based spreadsheet application being rewritten from C to Rust. The original C source files remain in `src/` as reference. The Rust rewrite is in early stages (Phase 0-1 of the plan in TODO.md) — core data structures and UI scaffolding exist but the parser, evaluator, and file I/O are stubs.

## Build Commands

```bash
cargo build          # Build the project
cargo run            # Run the application
cargo run -- file.tp # Open a spreadsheet file
cargo test           # Run tests (few tests exist currently)
cargo clippy         # Lint
```

## Architecture

The project produces both a library (`teapotlib`) and a binary (`teapot`).

### Module Structure

- **`main.rs`** — CLI entry point (clap). Handles argument parsing, batch mode command dispatch, and launches the interactive UI.
- **`lib.rs`** — Library root, re-exports all modules.
- **`sheet.rs`** — Core data model: `Sheet` (HashMap-based 3D grid), `Cell` (value, formula, formatting, clock flags), `Adjust` enum, `Direction` enum. All coordinates are `(x, y, z)` tuples where z is the sheet layer.
- **`token.rs`** — `Token` enum representing cell values and expression components (Empty, Integer, Float, String, Error, Location, Operator, etc.).
- **`parser.rs`** — Expression parsing (stub). Will be recursive descent.
- **`eval.rs`** — Expression evaluation and type operations (stub).
- **`display.rs`** — Terminal UI using ratatui/crossterm. Three input modes: Normal, Editing, Command. Renders spreadsheet grid, status bar, and input line.
- **`utils.rs`** — Helper functions.
- **`fileio/`** — File I/O submodule with format-specific files:
  - `fileio.rs` — Format detection and dispatch (by extension: `.tpa`, `.xdr`, `.csv`, `.sc`, `.wk1`)
  - `html.rs`, `latex.rs`, `context.rs` — Export formats (stubs)
  - `sc.rs`, `wk1.rs` — Import formats (stubs)

### Key Design Decisions

- **3D coordinates**: All cell positions use `(x, y, z)` — column, row, sheet layer.
- **Sparse storage**: Cells stored in `HashMap<(usize, usize, usize), Cell>` — only occupied cells consume memory.
- **Label cache**: `Sheet` maintains a separate `HashMap<String, (usize, usize, usize)>` for fast label-to-coordinate lookups.
- **Clock system**: Cells support iterative/clocked expressions via `clock_t0`/`t1`/`t2` flags.
- **Error handling**: Uses `anyhow::Result` throughout.

## Current State & What Needs Work

The Rust code is mid-migration from a previous AI agent's work. Core data structures (`Sheet`, `Cell`, `Token`) are implemented. The display module has a working ratatui UI. Most other modules are stubs that need real implementations — especially the expression parser/evaluator and all file I/O formats. Refer to TODO.md for the phased migration plan.
