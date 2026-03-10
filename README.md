# Teapot — Table Editor And Planner

A terminal-based spreadsheet with 3D cells, iterative (clocked) expressions, and a modern TUI.

Teapot was originally written in C by Michael Haardt. This is a full rewrite in Rust using ratatui/crossterm, preserving the original's design philosophy — compact, type-safe, three-dimensional — while adding modern editing conveniences.

## Features

- **3D spreadsheet** — cells addressed as `@(x, y, z)` across multiple sheet layers
- **Type-safe formulas** — 41 built-in functions; adding a number and a string is an error, not a silent conversion
- **Named cells** — label any cell, reference it as `@("labelName")` or just `labelName` in formulas
- **Clocked expressions** — three-phase iterative evaluation for cellular automata, simulations, and circular dependencies
- **Multiple file formats** — native `.tp`/`.tpz`, CSV, XLSX, ODS/XLS (read), HTML/LaTeX/ConTeXt (export)
- **Modern editing** — readline shortcuts, command history, tab completion, cell picker, command palette
- **Block operations** — mark a range, then format, export, sort, mirror, fill, or copy/move as a block
- **Batch mode** — scriptable via stdin for automated calculations

## Quick Start

```bash
cargo build --release
cargo run                      # new empty spreadsheet
cargo run -- file.csv          # open a CSV
cargo run -- spreadsheet.tp    # open native format
cargo run -- data.xlsx         # open Excel file
echo -e "set 0,0 42\nprint 0,0" | cargo run -- -b   # batch mode
```

## Usage

See [USAGE.md](USAGE.md) for the full keyboard reference and command list.

### Quick Reference

| Key | Action |
|-----|--------|
| `hjkl` / arrows | Move cursor |
| `HJKL` | Page movement |
| `=` | Enter formula |
| `'` | Enter text (auto-quotes) |
| `0-9` | Quick number entry |
| `e` / `Enter` | Edit existing cell |
| `:` | Command mode |
| `/` or `F1` | Command palette |
| `?` | Help screen |
| `m` (twice) | Mark block |
| `y` / `p` | Yank / paste |
| `Ctrl+Z` / `Ctrl+Y` | Undo / redo |
| `[` / `]` | Previous / next sheet |
| `n` / `N` | Search next / previous |
| `C` | Clock tick |
| `Ctrl+Q` | Quit |

### Formula Examples

```
42                    # integer
3.14                  # float
"hello"               # string
1 + 2 * 3             # arithmetic (= 7)
@(0,1,0)              # value of cell at column 0, row 1, sheet 0
@(x()-1, y(), z())    # relative reference (cell to the left)
@("total")            # label-based reference
eval(@(0,0,0))        # re-evaluate a cell's formula
sum(&(0,0,0), &(5,0,0))  # sum of range (0,0,0) through (5,0,0)
```

## Building

Requires Rust 1.70+.

```bash
cargo build              # debug build
cargo build --release    # optimized build
cargo test               # run tests (73 tests)
cargo clippy             # lint
```

## Architecture

The project produces both a library (`teapotlib`) and a binary (`teapot`).

**Expression pipeline:** `scanner::scan()` -> `Vec<Token>` -> `parser::eval_tokens()` -> `Token` (result)

The parser evaluates during recursive descent (no AST), matching the original C design. `EvalContext` replaces C globals and carries the sheet reference plus current cell coordinates.

**Storage:** Sparse `HashMap<(usize, usize, usize), Cell>` — only non-empty cells use memory.

See [CLAUDE.md](CLAUDE.md) for detailed module descriptions.

## History

The original C teapot was written by Michael Haardt and featured ncurses and FLTK interfaces. This Rust rewrite replaces both with a single ratatui-based terminal UI. The original C source files remain in `src/` as reference. See [README.ORG](README.ORG) for the original project description.

## License

GPL-3.0-or-later (same as the original).
