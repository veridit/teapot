# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Teapot is a terminal-based spreadsheet application rewritten from C to Rust. The original C source files remain in `src/` as reference. The core expression pipeline (scan → parse → evaluate) is functional, along with portable ASCII (.tpa) and CSV file I/O, and an interactive ratatui terminal UI.

## Build Commands

```bash
cargo build              # Build the project
cargo run                # Run interactive spreadsheet
cargo run -- file.tpa    # Open a .tpa spreadsheet file
cargo run -- file.csv    # Open a CSV file
cargo test               # Run tests (64 tests)
cargo clippy             # Lint
```

## Architecture

The project produces both a library (`teapotlib`) and a binary (`teapot`).

### Expression Pipeline

`scanner::scan()` → `Vec<Token>` → `parser::eval_tokens()` → `Token` (result)

The parser is a recursive descent evaluator that evaluates on the fly (no AST). It calls `eval::` operations directly during parsing. `EvalContext` (in parser.rs) replaces C globals and carries the sheet reference plus current cell coordinates.

### Module Structure

- **`main.rs`** — CLI entry point (clap). File loading by extension, batch mode, interactive UI launch.
- **`token.rs`** — `Token` enum (Empty, Integer, Float, String, Error, Location, Identifier, LabelIdentifier, Operator) and `Operator` enum (16 operators).
- **`scanner.rs`** — Tokenizer. `scan(&str) -> Vec<Token>` and `print_tokens()` for serialization.
- **`parser.rs`** — Recursive descent evaluator. `eval_tokens(&[Token], &mut EvalContext) -> Token`. Precedence: relational < additive < multiplicative < power < primary.
- **`eval.rs`** — Arithmetic/comparison operations (add, sub, mul, div, pow, neg, lt, le, ge, gt, eq, ne, about_eq). Type promotion rules match C implementation.
- **`functions.rs`** — 42 built-in functions: math, string, cell references (@, &, x, y, z), aggregates (sum, n, min, max), type conversion, utility.
- **`sheet.rs`** — `Sheet` (HashMap-based 3D grid), `Cell`, `Adjust`, `Direction`. Key methods: `update()`, `eval_cell()`, `getvalue()`, `putcont()`, `findlabel()`, `cachelabels()`.
- **`display.rs`** — Terminal UI (ratatui/crossterm). Three input modes: Normal, Editing, Command. Editing scans input → stores tokens → runs update().
- **`fileio.rs`** — Portable ASCII (.tpa) load/save, CSV load/save, text export. Stubs for XDR, HTML, LaTeX, ConTeXt, SC, WK1.

### Key Design Decisions

- **3D coordinates**: All cell positions use `(x, y, z)` — column, row, sheet layer.
- **Sparse storage**: `HashMap<(usize, usize, usize), Cell>`.
- **No AST**: Parser evaluates during recursive descent, matching C design.
- **EvalContext**: Replaces C globals (upd_sheet, upd_x, upd_y, upd_z).
- **Take/put pattern**: `eval_cell()` takes contents out of HashMap to avoid borrow conflicts during evaluation, then puts them back.
- **Error handling**: `anyhow::Result` for I/O, `Token::Error` for expression errors.

## What Still Needs Work

- File formats: XDR, SC, WK1, HTML export, LaTeX export, ConTeXt export
- Sheet operations: insert/delete rows/cols, sort, copy/move blocks
- Full UI: all keyboard commands, menus, help system, mouse support
- Batch mode: most commands are stubs
- Refer to TODO.md for the phased migration plan
